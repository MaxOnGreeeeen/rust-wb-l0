SET search_path TO public;
DROP EXTENSION IF EXISTS "uuid-ossp";

CREATE EXTENSION "uuid-ossp" SCHEMA public;

CREATE TABLE IF NOT EXISTS orders (
    order_uid UUID PRIMARY KEY DEFAULT public.uuid_generate_v4() NOT NULL,
    track_number VARCHAR NOT NULL,
    entry VARCHAR NOT NULL,
    locale VARCHAR,
    internal_signature VARCHAR,
    customer_id VARCHAR NOT NULL,
    delivery_service VARCHAR,
    shardkey VARCHAR,
    sm_id INTEGER,
    date_created TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    oof_shard VARCHAR
);

CREATE TABLE IF NOT EXISTS delivery (
    delivery_id SERIAL PRIMARY KEY,
    order_uid UUID REFERENCES orders(order_uid) ON DELETE CASCADE,
    name VARCHAR NOT NULL,
    phone VARCHAR NOT NULL,
    zip VARCHAR NOT NULL,
    city VARCHAR NOT NULL,
    address VARCHAR NOT NULL,
    region VARCHAR NOT NULL,
    email VARCHAR NOT NULL
);

CREATE TABLE IF NOT EXISTS payment (
    payment_id SERIAL PRIMARY KEY,
    order_uid UUID REFERENCES orders(order_uid) ON DELETE CASCADE,
    transaction VARCHAR NOT NULL,
    request_id VARCHAR,
    currency VARCHAR NOT NULL,
    provider VARCHAR NOT NULL,
    amount INTEGER NOT NULL,
    payment_dt TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    bank VARCHAR NOT NULL,
    delivery_cost INTEGER NOT NULL,
    goods_total INTEGER NOT NULL,
    custom_fee INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS items (
    item_id SERIAL PRIMARY KEY,
    order_uid UUID REFERENCES orders(order_uid) ON DELETE CASCADE,
    chrt_id BIGINT NOT NULL,
    track_number VARCHAR NOT NULL,
    price INTEGER NOT NULL,
    rid VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    sale INTEGER NOT NULL,
    size VARCHAR NOT NULL,
    total_price INTEGER NOT NULL,
    nm_id BIGINT NOT NULL,
    brand VARCHAR NOT NULL,
    status INTEGER NOT NULL
);