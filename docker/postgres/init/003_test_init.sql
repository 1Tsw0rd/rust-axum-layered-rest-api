CREATE DATABASE axum_test;

-- axum_test database 연결
\connect axum_test

-- sql 파일 실행
\i /docker-entrypoint-initdb.d/001_init.sql
\i /docker-entrypoint-initdb.d/002_seed.sql