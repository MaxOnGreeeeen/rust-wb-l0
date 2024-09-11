use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct CreateOrderDTO {
    pub track_number: String,
    pub entry: String,
    pub delivery: DeliveryDTO,
    pub payment: PaymentDTO,
    pub items: Vec<OrderItemDTO>,
    pub locale: String,
    pub internal_signature: String,
    pub customer_id: String,
    pub delivery_service: String,
    pub sm_id: i32,
    pub shardkey: String,
    pub oof_shard: String,
}

#[derive(Serialize, Deserialize)]
pub struct GetOrderDTO {
    pub order_uid: String,
    pub track_number: String,
    pub entry: String,
    pub delivery: DeliveryDTO,
    pub payment: PaymentDTO,
    pub items: Vec<OrderItemDTO>,
    pub locale: String,
    pub internal_signature: String,
    pub customer_id: String,
    pub delivery_service: String,
    pub sm_id: i32,
    pub date_created: String,
    pub shardkey: String,
    pub oof_shard: String,
}
impl GetOrderDTO {
    pub fn from_row(
        row: &tokio_postgres::Row,
        payment: PaymentDTO,
        delivery: DeliveryDTO,
        order_items: Vec<OrderItemDTO>,
    ) -> GetOrderDTO {
        let custom_data: NaiveDateTime = row.get(9);
        let formatted_date = custom_data.and_utc().to_rfc3339();
        let order_uid: Uuid = row.get(0);

        return GetOrderDTO {
            order_uid: order_uid.to_string(),
            track_number: row.get(1),
            entry: row.get(2),
            delivery,
            payment,
            items: order_items,
            locale: row.get(3),
            internal_signature: row.get(4),
            customer_id: row.get(5),
            delivery_service: row.get(6),
            shardkey: row.get(7),
            sm_id: row.get(8),
            date_created: formatted_date,
            oof_shard: row.get(10),
        };
    }
}

#[derive(Serialize, Deserialize)]
pub struct OrderItemId {
    item_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct OrderItemDTO {
    pub chrt_id: i64,
    pub track_number: String,
    pub price: i32,
    pub rid: String,
    pub name: String,
    pub sale: i32,
    pub size: String,
    pub total_price: i32,
    pub nm_id: i64,
    pub brand: String,
    pub status: i32,
}

impl From<&tokio_postgres::Row> for OrderItemDTO {
    fn from(value: &tokio_postgres::Row) -> Self {
        Self {
            chrt_id: value.get(0),
            track_number: value.get(1),
            price: value.get(2),
            rid: value.get(3),
            name: value.get(4),
            sale: value.get(5),
            size: value.get(6),
            total_price: value.get(7),
            nm_id: value.get(8),
            brand: value.get(9),
            status: value.get(10),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct DeliveryDTO {
    pub name: String,
    pub phone: String,
    pub zip: String,
    pub city: String,
    pub address: String,
    pub region: String,
    pub email: String,
}

impl From<tokio_postgres::Row> for DeliveryDTO {
    fn from(value: tokio_postgres::Row) -> Self {
        Self {
            name: value.get(0),
            phone: value.get(1),
            zip: value.get(2),
            city: value.get(3),
            address: value.get(4),
            region: value.get(5),
            email: value.get(6),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct PaymentDTO {
    pub transaction: String,
    pub request_id: String,
    pub currency: String,
    pub provider: String,
    pub amount: i32,
    pub payment_dt: i64,
    pub bank: String,
    pub delivery_cost: i32,
    pub goods_total: i32,
    pub custom_fee: i32,
}

impl From<tokio_postgres::Row> for PaymentDTO {
    fn from(value: tokio_postgres::Row) -> Self {
        Self {
            transaction: value.get(0),
            request_id: value.get(1),
            currency: value.get(2),
            provider: value.get(3),
            amount: value.get(4),
            payment_dt: value.get(5),
            bank: value.get(6),
            delivery_cost: value.get(7),
            goods_total: value.get(8),
            custom_fee: value.get(9),
        }
    }
}
