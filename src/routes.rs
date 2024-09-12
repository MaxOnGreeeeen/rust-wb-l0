use std::sync::Arc;

use crate::{
    errors::{handle_db_error, handle_get_request_error, handle_transaction_error, AppError},
    schema::{DeliveryDTO, GetOrderDTO, Order, OrderItemDTO, PaymentDTO},
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

    // Создание order
    let created_order = match OrderService::create_one(&mut transaction, &body, &[]).await {
        Ok(order) => order,
        Err(err) => {
            return Err(handle_transaction_error(err, transaction, "Create order error").await);
        }
    };
    let created_order_uuid = created_order.order_uid;

    // Создание delivery
    let created_delivery =
        match DeliveryService::create_one(&mut transaction, &body.delivery, &[&created_order_uuid])
            .await
        {
            Ok(delivery) => delivery,
            Err(err) => {
                return Err(
                    handle_transaction_error(err, transaction, "Create delivery error").await,
                );
            }
        };

    // Создание payment
    let created_payment =
        match PaymentService::create_one(&mut transaction, &body.payment, &[&created_order_uuid])
            .await
        {
            Ok(payment) => payment,
            Err(err) => {
                return Err(
                    handle_transaction_error(err, transaction, "Create payment error").await,
                );
            }
        };

    // Создание items
    let created_order_items =
        match OrderItemsService::create_many(&mut transaction, &body.items, &[&created_order_uuid])
            .await
        {
            Ok(items) => items,
            Err(err) => {
                return Err(handle_transaction_error(err, transaction, "Create items error").await);
            }
        };

    let order = GetOrderDTO::from_order(
        created_order,
        created_payment,
        created_delivery,
        created_order_items,
    );

    data.cache
        .lock()
        .await
        .update_record(created_order_uuid, order);

    // Commit транзакции
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
    if let Some(cached_item) = data.cache.lock().await.get_record(id) {
        return Ok((StatusCode::OK, Json(cached_item.data)));
    }

    // Получение order
    let order_row = match OrderService::get_one_by_id(&mut client_db, id).await {
        Ok(row) => row,
        Err(err) => {
            return Err(handle_get_request_error(err, "Get order error").await);
        }
    };
    if order_row.is_empty() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Order not found!"})),
        ));
    }

    // Получение payment
    let payment_row = match PaymentService::get_one_by_id(&mut client_db, id).await {
        Ok(row) => row,
        Err(err) => {
            return Err(handle_get_request_error(err, "Payment query error").await);
        }
    };

    // Получение delivery
    let delivery_row = match DeliveryService::get_one_by_id(&mut client_db, id).await {
        Ok(row) => row,
        Err(err) => {
            return Err(handle_get_request_error(err, "Delivery query error").await);
        }
    };

    // Получение items
    let order_item_rows = match OrderItemsService::get_many_by_id(&mut client_db, id).await {
        Ok(rows) => rows,
        Err(err) => {
            return Err(handle_get_request_error(err, "Order items query error").await);
        }
    };

    let payment = PaymentDTO::from(payment_row);
    let delivery = DeliveryDTO::from(delivery_row);
    let order_items: Vec<OrderItemDTO> = order_item_rows
        .iter()
        .map(|item| OrderItemDTO::from(item))
        .collect();
    let order = GetOrderDTO::from_row(order_row, payment, delivery, order_items);

    info!("Get order {}", &id.to_string());

    return Ok((StatusCode::OK, Json(order)));
}

// Типаж описывающий структуру запроса на получение элмента
trait GetOneById {
    async fn get_one_by_id(
        client: &mut Client,
        id: Uuid,
    ) -> Result<tokio_postgres::Row, PostgresError>;
}

// Типаж описывающий структуру запроса на получение множества элементов
trait GetManyById {
    async fn get_many_by_id(
        client: &mut Client,
        id: Uuid,
    ) -> Result<Vec<tokio_postgres::Row>, PostgresError>;
}

// Типаж описывающий структуру запроса на создание элемента
trait CreateOne<T, R>
where
    R: From<tokio_postgres::Row>,
{
    async fn create_one(
        transaction: &mut Transaction<'_>,
        body: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<R, AppError>;
}

// Типаж описывающий структуру запроса на создание множества элемента
trait CreateMany<T, R>
where
    R: From<tokio_postgres::Row>,
{
    async fn create_many(
        transaction: &mut Transaction<'_>,
        body: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<R>, AppError>;
}

struct PaymentService();
impl GetOneById for PaymentService {
    async fn get_one_by_id(
        client: &mut Client,
        id: Uuid,
    ) -> Result<tokio_postgres::Row, PostgresError> {
        return client
            .query_one(
                "SELECT transaction, request_id, currency,
                             provider, amount, payment_dt,
                             bank, delivery_cost, goods_total, custom_fee
                           FROM payment WHERE order_uid = $1",
                &[&id],
            )
            .await;
    }
}
impl CreateOne<PaymentDTO, PaymentDTO> for PaymentService {
    async fn create_one(
        transaction: &mut Transaction<'_>,
        body: &PaymentDTO,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<PaymentDTO, AppError> {
        let create_payment_stmt = transaction
            .prepare(
                "INSERT INTO payment (
                        order_uid, transaction, request_id,
                        currency, provider, amount,
                        payment_dt, bank, delivery_cost,
                        goods_total, custom_fee
                 ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) 
                  RETURNING 
                        transaction, request_id, currency, provider, amount,
                        payment_dt, bank, delivery_cost,
                        goods_total, custom_fee",
            )
            .await?;

        let payment_row = transaction
            .query_one(
                &create_payment_stmt,
                &[
                    params[0],
                    &body.transaction,
                    &body.request_id,
                    &body.currency,
                    &body.provider,
                    &body.amount,
                    &body.payment_dt,
                    &body.bank,
                    &body.delivery_cost,
                    &body.goods_total,
                    &body.custom_fee,
                ],
            )
            .await?;

        Ok(PaymentDTO::from(payment_row))
    }
}

struct OrderService();
impl GetOneById for OrderService {
    async fn get_one_by_id(
        client: &mut Client,
        id: Uuid,
    ) -> Result<tokio_postgres::Row, PostgresError> {
        return client
            .query_one(
                "SELECT order_uid, track_number, entry, locale,
                        internal_signature, customer_id, delivery_service,
                        shardkey, sm_id, date_created, oof_shard
                        FROM orders WHERE order_uid = $1",
                &[&id],
            )
            .await;
    }
}
impl CreateOne<CreateOrderDTO, Order> for OrderService {
    async fn create_one(
        transaction: &mut Transaction<'_>,
        body: &CreateOrderDTO,
        _params: &[&(dyn ToSql + Sync)],
    ) -> Result<Order, AppError> {
        let create_order_stmt = transaction
            .prepare(
                "INSERT INTO orders (
              track_number, entry, locale,
              internal_signature, customer_id, delivery_service,
              shardkey, sm_id, oof_shard
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING 
              order_uid, track_number, entry, locale,
              internal_signature, customer_id, delivery_service,
              sm_id, date_created, shardkey, oof_shard",
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

        Ok(Order::from(create_order_row))
    }
}

struct OrderItemsService();
impl GetManyById for OrderItemsService {
    async fn get_many_by_id(
        client: &mut Client,
        id: Uuid,
    ) -> Result<Vec<tokio_postgres::Row>, PostgresError> {
        return client
            .query(
                "SELECT chrt_id, track_number, price,
                            rid, name, sale, size,
                            total_price, nm_id, brand, status
                           FROM items WHERE order_uid = $1",
                &[&id],
            )
            .await;
    }
}
impl CreateMany<Vec<OrderItemDTO>, OrderItemDTO> for OrderItemsService {
    async fn create_many(
        transaction: &mut Transaction<'_>,
        body: &Vec<OrderItemDTO>,
        _params: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<OrderItemDTO>, AppError> {
        let mut query = String::from(
            "INSERT INTO items (order_uid,
            chrt_id, track_number, price,
            rid, name, sale, size, total_price,
            nm_id, brand, status
        ) VALUES ",
        );
        let mut params: Vec<&(dyn ToSql + Sync)> = Vec::new();
        for (i, item) in body.iter().enumerate() {
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

            params.push(_params[0]);
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
        query.push_str(
            " RETURNING 
                    chrt_id, track_number, price,
                    rid, name, sale, size, total_price,
                    nm_id, brand, status
            ",
        );

        let rows = transaction.query(&query, &params).await?;
        let order_items: Vec<OrderItemDTO> =
            rows.iter().map(|item| OrderItemDTO::from(item)).collect();

        Ok(order_items)
    }
}

struct DeliveryService();
impl GetOneById for DeliveryService {
    async fn get_one_by_id(
        client: &mut Client,
        id: Uuid,
    ) -> Result<tokio_postgres::Row, PostgresError> {
        return client
            .query_one(
                "SELECT name, phone, zip, city, address, region, email
                            FROM delivery WHERE order_uid = $1",
                &[&id],
            )
            .await;
    }
}
impl CreateOne<DeliveryDTO, DeliveryDTO> for DeliveryService {
    async fn create_one(
        transaction: &mut Transaction<'_>,
        body: &DeliveryDTO,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<DeliveryDTO, AppError> {
        let create_delivery_stmt = transaction
            .prepare(
                "INSERT INTO delivery (
              order_uid, name, phone,
              zip, city, address,
              region, email
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING
              name, phone,
              zip, city, address,
              region, email",
            )
            .await?;

        let create_delivery_row = transaction
            .query_one(
                &create_delivery_stmt,
                &[
                    params[0],
                    &body.name,
                    &body.phone,
                    &body.zip,
                    &body.city,
                    &body.address,
                    &body.region,
                    &body.email,
                ],
            )
            .await?;

        Ok(DeliveryDTO::from(create_delivery_row))
    }
}
