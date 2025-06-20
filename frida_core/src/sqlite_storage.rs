pub struct SqliteStorage<T: Processible> {
    pool: SqlitePool,
    _phantom: PhantomData<T>,
}

#[async_trait]
impl<T: Processible> Storage<dyn Importable, T> for SqliteStorage<T> {
    // Generic implementation that can be extended with hooks
}
