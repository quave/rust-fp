use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EcomF2Order {
    pub billing_identity: BillingIdentity,
    #[serde(default)]
    pub customer_domain: Option<String>,
    #[serde(default)]
    pub customer_account: Option<CustomerAccount>,
    #[serde(default)]
    pub deviating_shipment_identity: Option<ShipmentIdentity>,
    pub id: String,
    pub order: OrderDetails,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub shipment: Option<ShipmentSummary>,
    #[serde(default)]
    pub tenant_name: Option<String>,
    #[serde(default)]
    pub custom_ids: Vec<String>,
    #[serde(default)]
    pub device_data: Option<DeviceData>,
    #[serde(default)]
    pub event_date: Option<DateTime<Utc>>,
}

impl EcomF2Order {
    pub fn schema_version() -> (i32, i32) {
        (1, 1)
    }

    pub fn created(&self) -> DateTime<Utc> {
        self.order.date
    }

    pub fn order_number(&self) -> String {
        self.order
            .source_id
            .clone()
            .unwrap_or_else(|| self.id.clone())
    }

    pub fn state(&self) -> Option<State> {
        self.shipment.as_ref().and_then(|summary| summary.state)
    }

    pub fn shipment_type(&self) -> Option<ShipmentType> {
        self.shipment.as_ref().and_then(|summary| summary.kind)
    }

    pub fn origin(&self) -> Option<OrderChannel> {
        self.order.channel
    }

    pub fn checkout_time(&self) -> Option<i64> {
        match (self.event_date, Some(self.order.date)) {
            (Some(report), Some(created)) => {
                let diff = report.signed_duration_since(created).num_seconds();
                (diff >= 0).then_some(diff)
            }
            _ => None,
        }
    }

    pub fn referrer(&self) -> Option<String> {
        self.order.channel_detail.clone()
    }

    pub fn report_date(&self) -> Option<DateTime<Utc>> {
        self.event_date
    }

    pub fn device_ident_site(&self) -> Option<String> {
        self.device_data
            .as_ref()
            .and_then(|data| data.smart_id.clone())
    }

    pub fn device_ident_token(&self) -> Option<String> {
        self.device_data
            .as_ref()
            .and_then(|data| data.exact_id.clone())
    }

    pub fn customer_email(&self) -> Option<String> {
        self.billing_identity
            .email_address
            .as_ref()
            .and_then(|email| email.email.clone())
    }

    pub fn customer_full_name(&self) -> Option<String> {
        self.billing_identity.full_name()
    }

    pub fn customer_phone_numbers(&self) -> Vec<String> {
        self.billing_identity
            .phone_numbers
            .iter()
            .filter_map(|entry| entry.phone_number.clone())
            .collect()
    }

    pub fn item_iter(&self) -> impl Iterator<Item = &OrderItem> {
        self.order.order_items.iter()
    }

    pub fn total_amount(&self) -> f64 {
        self.item_iter().map(OrderItem::total_price).sum()
    }

    pub fn item_prices(&self) -> Vec<f64> {
        self.item_iter().map(OrderItem::unit_price).collect()
    }

    pub fn item_names(&self) -> Vec<String> {
        self.item_iter()
            .filter_map(|item| item.name.clone())
            .collect()
    }

    pub fn item_categories(&self) -> Vec<String> {
        self.item_iter()
            .filter_map(|item| item.category.clone())
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillingIdentity {
    pub date_of_birth: Option<NaiveDate>,
    pub email_address: Option<EmailAddress>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub payment_details: Option<PaymentDetails>,
    #[serde(default)]
    pub phone_numbers: Vec<PhoneNumber>,
    pub source_id: Option<String>,
    pub address: Option<Address>,
}

impl BillingIdentity {
    pub fn full_name(&self) -> Option<String> {
        match (&self.first_name, &self.last_name) {
            (Some(first), Some(last)) => {
                let combined = format!("{} {}", first, last).trim().to_string();
                if combined.is_empty() {
                    None
                } else {
                    Some(combined)
                }
            }
            (Some(first), None) if !first.trim().is_empty() => Some(first.clone()),
            (None, Some(last)) if !last.trim().is_empty() => Some(last.clone()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailAddress {
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentDetails {
    pub method: Option<PaymentMethod>,
    #[serde(rename = "type")]
    pub kind: Option<PaymentDetailType>,
}

impl PaymentDetails {
    pub fn identifier(&self) -> Option<String> {
        self.method.map(|method| method.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhoneNumber {
    pub last_updated_date: Option<DateTime<Utc>>,
    pub phone_number: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    #[serde(default)]
    pub first_name: Option<String>,
    #[serde(default)]
    pub last_name: Option<String>,
    #[serde(default)]
    pub street: Option<String>,
    #[serde(default)]
    pub house_number: Option<String>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub postal_code: Option<String>,
    #[serde(default)]
    pub country: Option<Country>,
    #[serde(default)]
    pub address_suffix: Option<String>,
    #[serde(default)]
    pub parcel_shop: Option<bool>,
    #[serde(default)]
    pub address_type: Option<AddressType>,
    #[serde(default)]
    pub latitude: Option<f64>,
    #[serde(default)]
    pub longitude: Option<f64>,
    #[serde(default)]
    pub door: Option<String>,
    #[serde(default)]
    pub floor: Option<String>,
    #[serde(default)]
    pub building: Option<String>,
    #[serde(default)]
    pub zip_code: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub company_name: Option<String>,
    #[serde(default)]
    pub last_updated_date: Option<DateTime<Utc>>,
}

impl Address {
    pub fn single_line(&self) -> String {
        let mut parts = Vec::new();
        if let Some(street) = self.street.as_ref() {
            let mut line = street.clone();
            if let Some(house) = &self.house_number {
                if !house.is_empty() {
                    line = format!("{} {}", line, house);
                }
            }
            if !line.trim().is_empty() {
                parts.push(line.trim().to_string());
            }
        }
        if let Some(postal) = &self.postal_code {
            if !postal.trim().is_empty() {
                parts.push(postal.trim().to_string());
            }
        }
        if let Some(city) = &self.city {
            if !city.trim().is_empty() {
                parts.push(city.trim().to_string());
            }
        }
        if let Some(country) = self.country.as_ref().and_then(enum_name) {
            parts.push(country);
        }
        parts.join(", ")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerAccount {
    pub created_date: Option<DateTime<Utc>>,
    pub first_order_date: Option<DateTime<Utc>>,
    pub last_payment_date: Option<DateTime<Utc>>,
    pub last_updated_password_date: Option<DateTime<Utc>>,
    pub number_of_past_orders: Option<i64>,
    pub open_balance: Option<String>,
    pub source_id: Option<String>,
    #[serde(rename = "type")]
    pub account_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShipmentIdentity {
    pub address: Option<Address>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    #[serde(default)]
    pub phone_numbers: Vec<PhoneNumber>,
    pub source_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderDetails {
    pub channel: Option<OrderChannel>,
    pub channel_detail: Option<String>,
    pub date: DateTime<Utc>,
    #[serde(default)]
    pub order_items: Vec<OrderItem>,
    pub source_id: Option<String>,
    pub total_price: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderItem {
    pub category: Option<String>,
    pub characteristic: Option<String>,
    pub name: Option<String>,
    pub price_per_item: Option<String>,
    pub quantity: Option<i64>,
    pub shipment: Option<ShipmentSummary>,
    pub source_id: Option<String>,
}

impl OrderItem {
    pub fn unit_price(&self) -> f64 {
        self.price_per_item
            .as_ref()
            .and_then(|value| parse_decimal(value))
            .unwrap_or(0.0)
    }

    pub fn quantity(&self) -> f64 {
        self.quantity.map(|qty| qty as f64).unwrap_or(1.0)
    }

    pub fn total_price(&self) -> f64 {
        self.unit_price() * self.quantity().max(0.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShipmentSummary {
    pub state: Option<State>,
    #[serde(rename = "type")]
    pub kind: Option<ShipmentType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceData {
    pub exact_id: Option<String>,
    pub smart_id: Option<String>,
}

fn parse_decimal(value: &str) -> Option<f64> {
    let normalized = value.replace(',', ".");
    normalized.parse::<f64>().ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    New,
    Invoiced,
    Shipped,
    Blocked,
    Canceled,
    Returned,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ShipmentType {
    Normal,
    Express,
    #[serde(rename = "12h")]
    TwelveHour,
    DesiredDate,
    MobileVoucher,
    OnSite,
    PrintVoucher,
    SinglePackage,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OrderChannel {
    Internet,
    Phone,
    Mail,
    PartnerWebsite,
    Shop,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PaymentMethod {
    Invoice,
    CreditCard,
    DirectDebit,
    PayPal,
    #[serde(other)]
    Other,
}

impl fmt::Display for PaymentMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = enum_name(self).unwrap_or_else(|| "unknown".to_string());
        write!(f, "{value}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PaymentDetailType {
    ActivePayment,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AddressType {
    Shipment,
    Customer,
    CreditAgency,
    Reseller,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DunningLevel {
    FirstReminder,
    SecondReminder,
    ThirdReminder,
    PreDebtCollection,
    DebtCollection,
    SuccessfulDebtCollection,
    UnsuccessfulDebtCollection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PaymentStatus {
    Unpaid,
    Paid,
    PartlyPaid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Country {
    Af,
    Ax,
    Al,
    Dz,
    As,
    Ad,
    Ao,
    Ai,
    Aq,
    Ag,
    Ar,
    Am,
    Aw,
    Au,
    At,
    Az,
    Bs,
    Bh,
    Bd,
    Bb,
    By,
    Be,
    Bz,
    Bj,
    Bm,
    Bt,
    Bo,
    Ba,
    Bw,
    Bv,
    Br,
    Io,
    Bn,
    Bg,
    Bf,
    Bi,
    Kh,
    Cm,
    Ca,
    Cv,
    Ky,
    Cf,
    Td,
    Cl,
    Cn,
    Cx,
    Cc,
    Co,
    Km,
    Cg,
    Cd,
    Ck,
    Cr,
    Ci,
    Hr,
    Cu,
    Cy,
    Cz,
    Dk,
    Dj,
    Dm,
    Do,
    Ec,
    Eg,
    Sv,
    Gq,
    Er,
    Ee,
    Et,
    Fk,
    Fo,
    Fj,
    Fi,
    Fr,
    Gf,
    Pf,
    Tf,
    Ga,
    Gm,
    Ge,
    De,
    Gh,
    Gi,
    Gr,
    Gl,
    Gd,
    Gp,
    Gu,
    Gt,
    Gg,
    Gn,
    Gw,
    Gy,
    Ht,
    Hm,
    Va,
    Hn,
    Hk,
    Hu,
    Is,
    In,
    Id,
    Ir,
    Iq,
    Ie,
    Im,
    Il,
    It,
    Jm,
    Jp,
    Je,
    Jo,
    Kz,
    Ke,
    Ki,
    Kr,
    Kw,
    Kg,
    La,
    Lv,
    Lb,
    Ls,
    Lr,
    Ly,
    Li,
    Lt,
    Lu,
    Mo,
    Mk,
    Mg,
    Mw,
    My,
    Mv,
    Ml,
    Mt,
    Mh,
    Mq,
    Mr,
    Mu,
    Yt,
    Mx,
    Fm,
    Md,
    Mc,
    Mn,
    Me,
    Ms,
    Ma,
    Mz,
    Mm,
    Na,
    Nr,
    Np,
    Nl,
    An,
    Nc,
    Nz,
    Ni,
    Ne,
    Ng,
    Nu,
    Nf,
    Mp,
    No,
    Om,
    Pk,
    Pw,
    Ps,
    Pa,
    Pg,
    Py,
    Pe,
    Ph,
    Pn,
    Pl,
    Pt,
    Pr,
    Qa,
    Re,
    Ro,
    Ru,
    Rw,
    Bl,
    Sh,
    Kn,
    Lc,
    Mf,
    Pm,
    Vc,
    Ws,
    Sm,
    St,
    Sa,
    Sn,
    Rs,
    Sc,
    Sl,
    Sg,
    Sk,
    Si,
    Sb,
    So,
    Za,
    Gs,
    Es,
    Lk,
    Sd,
    Sr,
    Sj,
    Sz,
    Se,
    Ch,
    Sy,
    Tw,
    Tj,
    Tz,
    Th,
    Tl,
    Tg,
    Tk,
    To,
    Tt,
    Tn,
    Tr,
    Tm,
    Tc,
    Tv,
    Ug,
    Ua,
    Ae,
    Gb,
    Us,
    Um,
    Uy,
    Uz,
    Vu,
    Ve,
    Vn,
    Vg,
    Vi,
    Wf,
    Eh,
    Ye,
    Zm,
    Zw,
}

pub fn enum_name<T>(value: &T) -> Option<String>
where
    T: Serialize,
{
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
}
