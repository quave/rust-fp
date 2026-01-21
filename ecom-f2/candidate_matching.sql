-- =========================================================
-- Graph storage for PostgreSQL (transactions + attributes)
-- - Nodes: transactions
-- - Edges: between transactions that share any attribute value
-- - Confidence is per attribute type (0..100)
-- - For traversal, each pair uses the highest-confidence attribute
-- - Denormalized adjacency for simple/fast queries
-- =========================================================

-- Optional: run inside a transaction
-- begin;

-- =========================================================
-- Core tables
-- =========================================================

-- One row per transaction (node)
create table if not exists transactions (
  txn_id bigserial primary key
  -- add other columns as needed
);

-- Attribute types with global confidences
create table if not exists connection_type (
  attr_id smallserial primary key,
  name text not null unique,
  confidence smallint not null check (confidence between 0 and 100)
);

-- Distinct values per attribute type
create table if not exists attribute_value (
  attr_val_id bigserial primary key,
  attr_id smallint not null references connection_type(attr_id),
  value text not null,
  unique (attr_id, value)
);

-- Which transaction has which attribute value(s)
create table if not exists transaction_attribute (
  txn_id bigint not null references transactions(txn_id) on delete cascade,
  attr_val_id bigint not null references attribute_value(attr_val_id) on delete cascade,
  primary key (txn_id, attr_val_id)
);

create index if not exists idx_ta_attrval_txn on transaction_attribute (attr_val_id, txn_id);
create index if not exists idx_ta_txn on transaction_attribute (txn_id);
create index if not exists idx_av_attr on attribute_value (attr_id);

-- =========================================================
-- Denormalized edge stores
-- =========================================================

-- Undirected summary: one row per unordered pair (a_id < b_id)
create table if not exists edges_undirected (
  a_id bigint not null references transactions(txn_id) on delete cascade,
  b_id bigint not null references transactions(txn_id) on delete cascade,
  attr_ids smallint[] not null,                          -- attribute types that connect this pair
  best_attr_id smallint not null references connection_type(attr_id),
  best_conf smallint not null,                           -- equals confidence of best_attr_id
  attrs_json jsonb,                                      -- e.g., [{name, confidence}, ...]
  primary key (a_id, b_id),
  check (a_id < b_id)
);

create index if not exists idx_edges_u_a on edges_undirected (a_id);
create index if not exists idx_edges_u_b on edges_undirected (b_id);

-- Directed adjacency: two rows per undirected edge (src->dst and dst->src)
create table if not exists adjacency_directed (
  src_id bigint not null references transactions(txn_id) on delete cascade,
  dst_id bigint not null references transactions(txn_id) on delete cascade,
  attr_ids smallint[] not null,
  best_attr_id smallint not null references connection_type(attr_id),
  best_attr_name text not null,
  best_conf smallint not null,
  attrs_json jsonb,
  primary key (src_id, dst_id)
);

create index if not exists idx_adj_src on adjacency_directed (src_id);

-- =========================================================
-- Incremental maintenance function:
-- Refresh/insert edges for a given transaction after its attributes are inserted
-- =========================================================

create or replace function refresh_edges_for_txn(p_txn_id bigint)
returns void
language sql
as $$
with pairs as (
  -- For the new txn, find all other txns sharing any attribute value, grouped by attribute type
  select
    least(p_txn_id, o.txn_id) as a_id,
    greatest(p_txn_id, o.txn_id) as b_id,
    av.attr_id
  from transaction_attribute ta_new
  join transaction_attribute o
    on o.attr_val_id = ta_new.attr_val_id
   and o.txn_id <> p_txn_id
  join attribute_value av on av.attr_val_id = ta_new.attr_val_id
  where ta_new.txn_id = p_txn_id
  group by 1, 2, av.attr_id
),
pair_agg as (
  -- Collapse to one row per pair with set of attribute types
  select a_id, b_id, array_agg(attr_id order by attr_id) as attr_ids
  from pairs
  group by a_id, b_id
),
upsert_edges as (
  -- Upsert edges_undirected, merge attr sets, recompute best_attr/best_conf/json
  insert into edges_undirected (a_id, b_id, attr_ids, best_attr_id, best_conf, attrs_json)
  select
    p.a_id,
    p.b_id,
    p.attr_ids,
    (
      select at.attr_id
      from unnest(p.attr_ids) x(attr_id)
      join connection_type at using (attr_id)
      order by at.confidence desc, at.name asc
      limit 1
    ) as best_attr_id,
    (
      select max(at.confidence)
      from unnest(p.attr_ids) x(attr_id)
      join connection_type at using (attr_id)
    ) as best_conf,
    (
      select jsonb_agg(distinct jsonb_build_object('name', at.name, 'confidence', at.confidence)
                       order by at.confidence desc, at.name)
      from unnest(p.attr_ids) x(attr_id)
      join connection_type at using (attr_id)
    ) as attrs_json
  from pair_agg p
  on conflict (a_id, b_id) do update
  set attr_ids = (
        select array_agg(distinct a order by a)
        from unnest(array_cat(edges_undirected.attr_ids, excluded.attr_ids)) s(a)
      ),
      best_attr_id = (
        select at.attr_id
        from unnest(
          (select array_agg(distinct a order by a)
           from unnest(array_cat(edges_undirected.attr_ids, excluded.attr_ids)) s(a))
        ) x(attr_id)
        join connection_type at using (attr_id)
        order by at.confidence desc, at.name asc
        limit 1
      ),
      best_conf = (
        select max(at.confidence)
        from unnest(
          (select array_agg(distinct a)
           from unnest(array_cat(edges_undirected.attr_ids, excluded.attr_ids)) s(a))
        ) x(attr_id)
        join connection_type at using (attr_id)
      ),
      attrs_json = (
        select jsonb_agg(distinct jsonb_build_object('name', at.name, 'confidence', at.confidence)
                         order by at.confidence desc, at.name)
        from unnest(
          (select array_agg(distinct a)
           from unnest(array_cat(edges_undirected.attr_ids, excluded.attr_ids)) s(a))
        ) x(attr_id)
        join connection_type at using (attr_id)
      )
  returning a_id, b_id, attr_ids, best_attr_id, best_conf, attrs_json
)
-- Materialize directed adjacency for simple queries (both directions), upserting
insert into adjacency_directed (src_id, dst_id, attr_ids, best_attr_id, best_attr_name, best_conf, attrs_json)
select ue.a_id as src_id,
       ue.b_id as dst_id,
       ue.attr_ids,
       ue.best_attr_id,
       at.name as best_attr_name,
       ue.best_conf,
       ue.attrs_json
from upsert_edges ue
join connection_type at on at.attr_id = ue.best_attr_id
union all
select ue.b_id as src_id,
       ue.a_id as dst_id,
       ue.attr_ids,
       ue.best_attr_id,
       at.name as best_attr_name,
       ue.best_conf,
       ue.attrs_json
from upsert_edges ue
join connection_type at on at.attr_id = ue.best_attr_id
on conflict (src_id, dst_id) do update
set attr_ids = excluded.attr_ids,
    best_attr_id = excluded.best_attr_id,
    best_attr_name = excluded.best_attr_name,
    best_conf = excluded.best_conf,
    attrs_json = excluded.attrs_json;
$$;

-- =========================================================
-- Optional statement-level trigger to auto-refresh edges
-- after inserting transaction_attribute rows
-- (Requires PostgreSQL 10+ for REFERENCING NEW TABLE)
-- =========================================================

create or replace function ta_after_insert_stmt()
returns trigger
language plpgsql
as $$
declare r record;
begin
  for r in (select distinct txn_id from new_table) loop
    perform refresh_edges_for_txn(r.txn_id);
  end loop;
  return null;
end;
$$;

drop trigger if exists trg_ta_ai_stmt on transaction_attribute;

create trigger trg_ta_ai_stmt
after insert on transaction_attribute
referencing new table as new_table
for each statement
execute function ta_after_insert_stmt();

-- =========================================================
-- Query helpers
-- =========================================================

-- Q1: Direct neighbors function (all connection types per neighbor)
create or replace function get_neighbors(p_txn_id bigint)
returns table (
  neighbor_txn_id bigint,
  best_attr_name text,
  best_conf smallint,
  attr_ids smallint[],
  connection_types jsonb
)
language sql
as $$
select
  a.dst_id as neighbor_txn_id,
  a.best_attr_name,
  a.best_conf,
  a.attr_ids,
  a.attrs_json as connection_types
from adjacency_directed a
where a.src_id = p_txn_id
order by a.best_conf desc, a.dst_id;
$$;

-- Q2: Full reachable set with one shortest path per node (by hop count),
-- returning the path of attribute names used along the way.
create or replace function get_graph_tree(p_root_id bigint)
returns table (
  txn_id bigint,
  parent_txn_id bigint,
  edge_attribute text,
  depth integer,
  path_nodes bigint[],
  path_attrs text[]
)
language sql
as $$
with recursive walk as (
  -- Start at root
  select
    p_root_id::bigint as node,
    null::bigint as parent,
    null::text as via_attr,
    array[p_root_id::bigint]::bigint[] as path_nodes,
    array[]::text[] as path_attrs,
    0 as depth
  union all
  -- Expand frontier, avoid revisiting nodes already in this path
  select
    a.dst_id as node,
    w.node as parent,
    a.best_attr_name as via_attr,
    w.path_nodes || a.dst_id,
    w.path_attrs || a.best_attr_name,
    w.depth + 1
  from walk w
  join adjacency_directed a on a.src_id = w.node
  where not (a.dst_id = any(w.path_nodes))
),
min_depth as (
  -- Choose the fewest-hop path per node
  select node, min(depth) as depth
  from walk
  group by node
)
select
  w.node as txn_id,
  w.parent as parent_txn_id,
  w.via_attr as edge_attribute,
  w.depth,
  w.path_nodes,
  w.path_attrs
from walk w
join min_depth m
  on m.node = w.node and m.depth = w.depth
order by w.depth, w.node;
$$;

-- =========================================================
-- Usage examples
-- =========================================================

-- 1) Insert attribute types (with confidences)
-- insert into connection_type (name, confidence) values
--   ('email', 92), ('phone', 78), ('address', 40);

-- 2) Insert transactions and their attributes:
-- insert into transactions default values returning txn_id;
-- insert into attribute_value (attr_id, value) values (1, 'a@example.com') on conflict do nothing;
-- insert into transaction_attribute (txn_id, attr_val_id) values (...);
-- The trigger will call refresh_edges_for_txn automatically after each insert statement.
-- Alternatively, call it manually once after loading a txn's attributes:
-- select refresh_edges_for_txn(<txn_id>);

-- 3) Direct neighbors:
-- select * from get_neighbors(<txn_id>);

-- 4) Full reachable graph (one shortest path per node):
-- select * from get_graph_tree(<txn_id>);

-- commit;