use std::sync::Arc;

use crate::{
    errors::{
        handle_create_delivery_error, handle_create_order_error, handle_create_order_items,
        handle_create_payment_error, handle_db_error, AppError,
    },
    schema::{DeliveryDTO, GetOrderDTO, OrderItemDTO, PaymentDTO},
};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use log::{error, info};
use serde_json::json;
use tokio_postgres::{types::ToSql, Client, Error as PostgresError, Transaction};
use uuid::Uuid;

use crate::{schema::CreateOrderDTO, AppState};

// POST /api/orders/
// Endpoint для создания заказа
pub async fn create_order_handler(
    State(data): State<Arc<AppState>>,
    Json(body): Json<CreateOrderDTO>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let mut client_db = data.db.lock().await;

    let mut transaction = match client_db.transaction().await {
        Ok(tx) => tx,
        Err(err) => return Err(handle_db_error(err)),
    };

    let created_order_uuid = match create_order(&mut transaction, &body).await {
        Ok(order_uuid) => order_uuid,
        Err(err) => {
            if let Err(rollback_err) = transaction.rollback().await {
                error!("Failed to rollback transaction: {:?}", rollback_err);
            }
            error!("Create order error: {}", err);

            return Err(handle_create_order_error(err));
        }
    };

    let create_delivery_res =
        create_delivery(&mut transaction, &body.delivery, &created_order_uuid).await;
    if let Err(err) = create_delivery_res {
        if let Err(rollback_err) = transaction.rollback().await {
            error!(
                "Failed to rollback transaction after delivery error: {:?}",
                rollback_err
            );
        }
        error!("Create delivery error");

        return Err(handle_create_delivery_error(err));
    }

    let create_payment_res =
        create_payment(&mut transaction, &body.payment, &created_order_uuid).await;
    if let Err(err) = create_payment_res {
        if let Err(rollback_err) = transaction.rollback().await {
            error!(
                "Failed to rollback transaction after payment error: {:?}",
                rollback_err
            );
        }
        error!("Create payment error, {}", err);

        return Err(handle_create_payment_error(err));
    }

    let create_order_items_res =
        create_order_items(&mut transaction, &body.items, &created_order_uuid).await;
    if let Err(err) = create_order_items_res {
        if let Err(rollback_err) = transaction.rollback().await {
            error!(
                "Failed to rollback transaction after create items error error: {:?}",
                rollback_err
            );
        }
        error!("Create items error: {err}");

        return Err(handle_create_order_items(err));
    }

    transaction.commit().await.map_err(|err| {
        error!("Failed to commit transaction: {:?}", err);

        let error_response = serde_json::json!({
            "status": "error",
            "message": "Failed to commit transaction"
        });
        (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response))
    })?;

    info!("Order {} created", created_order_uuid);

    return Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "order_uid": &created_order_uuid,
        })),
    ));
}

// GET /api/orders/:id
// Endpoint для получения заказа по id
pub async fn get_order_handler(
    Path(id): Path<uuid::Uuid>,
    State(data): State<Arc<AppState>>,
) -> Result<(StatusCode, Json<GetOrderDTO>), (StatusCode, Json<serde_json::Value>)> {
    let mut client_db = data.db.lock().await;

    let order_row = match get_order_row(&mut client_db, id).await {
        Ok(row) => row,
        Err(err) => {
            error!("Order query error: {err}");

            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Order query error"})),
            ));
        }
    };
    if order_row.is_empty() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Order not found!"})),
        ));
    }

    let payment_row = match get_payment_row(&mut client_db, id).await {
        Ok(row) => row,
        Err(err) => {
            error!("Payment query error: {err}");

            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Payment query erro"})),
            ));
        }
    };

    let delivery_row = match get_deliver_row(&mut client_db, id).await {
        Ok(row) => row,
        Err(err) => {
            error!("Delivery query error: {err}");

            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Delivery query erro"})),
            ));
        }
    };

    let order_item_rows = match get_item_rows(&mut client_db, id).await {
        Ok(row) => row,
        Err(err) => {
            error!("Order items query error: {err}");

            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Order items query error"})),
            ));
        }
    };

    let payment = PaymentDTO::from(payment_row);
    let delivery = DeliveryDTO::from(delivery_row);
    let order_items: Vec<OrderItemDTO> = order_item_rows
        .iter()
        .map(|item| OrderItemDTO::from(item))
        .collect();
    let order = GetOrderDTO::from_row(&order_row, payment, delivery, order_items);

    info!("Get order {}", &id.to_string());

    return Ok((StatusCode::OK, Json(order)));
}

async fn get_payment_row(
    client: &mut Client,
    uuid: Uuid,
) -> Result<tokio_postgres::Row, PostgresError> {
    return client.query_one(
        "SELECT transaction, request_id, currency, provider, amount, payment_dt, bank, delivery_cost, goods_total, custom_fee
         FROM payment WHERE order_uid = $1",
        &[&uuid],
    ).await;
}

async fn get_order_row(
    client: &mut Client,
    uuid: Uuid,
) -> Result<tokio_postgres::Row, PostgresError> {
    return client
        .query_one(
            "SELECT order_uid, track_number, entry, locale,
        internal_signature, customer_id, delivery_service,
        shardkey, sm_id, date_created, oof_shard
         FROM orders WHERE order_uid = $1",
            &[&uuid],
        )
        .await;
}

async fn get_item_rows(
    client: &mut Client,
    uuid: Uuid,
) -> Result<Vec<tokio_postgres::Row>, PostgresError> {
    return client
        .query(
            "SELECT chrt_id, track_number, price,
             rid, name, sale, size,
             total_price, nm_id, brand, status
         FROM items WHERE order_uid = $1",
            &[&uuid],
        )
        .await;
}

async fn get_deliver_row(
    client: &mut Client,
    uuid: Uuid,
) -> Result<tokio_postgres::Row, PostgresError> {
    return client
        .query_one(
            "SELECT name, phone, zip, city, address, region, email
         FROM delivery WHERE order_uid = $1",
            &[&uuid],
        )
        .await;
}

// Добавление элементов заказа
async fn create_order_items(
    transaction: &mut Transaction<'_>,
    items: &Vec<OrderItemDTO>,
    order_uid: &Uuid,
) -> Result<(), PostgresError> {
    let mut query = String::from(
        "INSERT INTO items (order_uid,
            chrt_id, track_number, price,
            rid, name, sale, size, total_price,
            nm_id, brand, status
        ) VALUES ",
    );
    let mut params: Vec<&(dyn ToSql + Sync)> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let param_start = i * 12 + 1;
        query.push_str(&format!(
            "(${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}),",
            param_start,      // order_uid
            param_start + 1,  // chrt_id
            param_start + 2,  // track_number
            param_start + 3,  // price
            param_start + 4,  // rid
            param_start + 5,  // name
            param_start + 6,  // sale
            param_start + 7,  // size
            param_start + 8,  // total_price
            param_start + 9,  // nm_id
            param_start + 10, // brand
            param_start + 11, // status
        ));

        params.push(&order_uid);
        params.push(&item.chrt_id);
        params.push(&item.track_number);
        params.push(&item.price);
        params.push(&item.rid);
        params.push(&item.name);
        params.push(&item.sale);
        params.push(&item.size);
        params.push(&item.total_price);
        params.push(&item.nm_id);
        params.push(&item.brand);
        params.push(&item.status);
    }
    query.pop();

    transaction.query(&query, &params).await?;

    Ok(())
}

// Создание заказа
async fn create_order(
    transaction: &mut Transaction<'_>,
    body: &CreateOrderDTO,
) -> Result<Uuid, AppError> {
    let create_order_stmt = transaction
        .prepare(
            "INSERT INTO orders (
              track_number, entry, locale,
              internal_signature, customer_id, delivery_service,
              shardkey, sm_id, oof_shard
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING order_uid::varchar",
        )
        .await?;

    let create_order_row = transaction
        .query_one(
            &create_order_stmt,
            &[
                &body.track_number,
                &body.entry,
                &body.locale,
                &body.internal_signature,
                &body.customer_id,
                &body.delivery_service,
                &body.shardkey,
                &body.sm_id,
                &body.oof_shard,
            ],
        )
        .await?;

    let order_uid_str: &str = create_order_row.get(0);
    let order_uid = match Uuid::parse_str(order_uid_str) {
        Ok(uid) => uid,
        Err(err) => {
            return Err(err)?;
        }
    };
    Ok(order_uid)
}

// Создание доставки заказа
async fn create_delivery(
    transaction: &mut Transaction<'_>,
    delivery: &DeliveryDTO,
    order_uid: &Uuid,
) -> Result<(), PostgresError> {
    let create_delivery_stmt = transaction
        .prepare(
            "INSERT INTO delivery (
              order_uid, name, phone,
              zip, city, address,
              region, email
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING delivery_id",
        )
        .await?;

    transaction
        .query_one(
            &create_delivery_stmt,
            &[
                order_uid,
                &delivery.name,
                &delivery.phone,
                &delivery.zip,
                &delivery.city,
                &delivery.address,
                &delivery.region,
                &delivery.email,
            ],
        )
        .await?;

    Ok(())
}

// Создание оплаты заказа
async fn create_payment(
    transaction: &mut Transaction<'_>,
    payment: &PaymentDTO,
    order_uid: &Uuid,
) -> Result<(), PostgresError> {
    let create_payment_stmt = transaction
        .prepare(
            "INSERT INTO payment (
                        order_uid, transaction, request_id,
                        currency, provider, amount,
                        payment_dt, bank, delivery_cost,
                        goods_total, custom_fee
                 ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING payment_id",
        )
        .await?;

    transaction
        .execute(
            &create_payment_stmt,
            &[
                order_uid,
                &payment.transaction,
                &payment.request_id,
                &payment.currency,
                &payment.provider,
                &payment.amount,
                &payment.payment_dt,
                &payment.bank,
                &payment.delivery_cost,
                &payment.goods_total,
                &payment.custom_fee,
            ],
        )
        .await?;

    Ok(())
}
