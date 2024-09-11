use log::info;
use reqwest::Client;
use std::{sync::Arc, time::Duration};
use uuid::Uuid;

use tokio_postgres::Error;

use crate::schema::{CreateOrderDTO, DeliveryDTO, OrderItemDTO, PaymentDTO};

// Создание единичного заказа
async fn create_order(port: u16, client: &Client, order: &CreateOrderDTO) {
    let url = format!("http://localhost:{}/api/orders", port);

    match client.post(url).json(order).send().await {
        Ok(response) => {
            if response.status().is_success() {
                println!(
                    "Order created successfully: {:?}",
                    response.text().await.unwrap()
                );
            } else {
                println!(
                    "Failed to create order: {:?}",
                    response.text().await.unwrap()
                );
            }
        }
        Err(err) => {
            println!("Error sending request: {:?}", err);
        }
    }
}

async fn bulk_create_orders(args: Arc<crate::Args>) {
    let client = Client::new();
    for _ in 0..args.count {
        let order = CreateOrderDTO {
            track_number: "TN123456789".to_string(),
            entry: "warehouse".to_string(),
            locale: "en_US".to_string(),
            internal_signature: "sig12345".to_string(),
            customer_id: Uuid::new_v4().to_string(),
            delivery_service: "DHL".to_string(),
            shardkey: "sk123".to_string(),
            sm_id: 1,
            oof_shard: "shard1".to_string(),
            delivery: DeliveryDTO {
                name: "John Doe".to_string(),
                phone: "555-1234".to_string(),
                zip: "12345".to_string(),
                city: "Sample City".to_string(),
                address: "1234 Sample Street".to_string(),
                region: "Sample Region".to_string(),
                email: "john.doe@example.com".to_string(),
            },
            payment: PaymentDTO {
                transaction: "tx12345".to_string(),
                request_id: "rq12345".to_string(),
                currency: "USD".to_string(),
                provider: "Visa".to_string(),
                amount: 100,
                payment_dt: 1637924400, // Unix timestamp
                bank: "Sample Bank".to_string(),
                delivery_cost: 5,
                goods_total: 95,
                custom_fee: 0,
            },
            items: vec![OrderItemDTO {
                chrt_id: 123456789,
                track_number: "TN123456789".to_string(),
                price: 100,
                rid: "RID12345".to_string(),
                name: "Sample Item".to_string(),
                sale: 10,
                size: "M".to_string(),
                total_price: 90,
                nm_id: 987654321,
                brand: "Sample Brand".to_string(),
                status: 1,
            }],
        };

        // Отправляем запрос на создание заказа
        create_order(args.port, &client, &order).await;

        // Задержка между запросами (например, 1 секунда)
        tokio::time::sleep(Duration::from_millis(args.delay)).await;
    }

    info!("Created: {} Order", &args.count);
}

// Функция для тестирования работы API
pub async fn fill_test_data(args: Arc<crate::Args>) -> Result<(), Error> {
    bulk_create_orders(args).await;
    Ok(())
}
