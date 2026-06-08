# Trading Platform

一句话架构设计：项目采用 Axum + sqlx 分层架构，将 HTTP 路由、请求处理、业务服务、数据库访问、DTO、模型、错误与统一响应拆分，核心订单状态流转和资金结算全部在服务层事务内完成。

## 本地运行

1. 准备 Postgres 数据库，并在 `.env` 中配置连接字符串：

```env
DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres_db
```

2. 初始化数据库表和测试数据：

```bash
psql "$DATABASE_URL" -f db/init.sql
```

Windows PowerShell 可使用：

```powershell
psql $env:DATABASE_URL -f db/init.sql
```

3. 启动服务：

```bash
cargo run
```

服务默认监听 `127.0.0.1:3000`。

## API 示例

创建需求订单：

```bash
curl -X POST http://127.0.0.1:3000/orders \
  -H "Content-Type: application/json" \
  -d '{"client_id":1,"worker_id":3,"amount":10000,"description":"预约一次服务"}'
```

更新订单状态：

```bash
curl -X PATCH http://127.0.0.1:3000/orders/1/status \
  -H "Content-Type: application/json" \
  -d '{"target_status":"Accepted","operator_id":3}'
```

```bash
curl -X PATCH http://127.0.0.1:3000/orders/1/status \
  -H "Content-Type: application/json" \
  -d '{"target_status":"Completed","operator_id":1}'
```

## 核心规则

- 创建订单会检查 Client 可用余额是否足够；余额不足不能创建，但创建时只生成 `Pending`，不冻结资金。
- `Pending -> Accepted` 只能由订单指定 Worker 操作，并再次检查余额后冻结 Client 可用余额。
- `Accepted -> Completed` 只能由订单 Client 操作，Worker 入账 `amount - amount / 10`，平台手续费记录入账 `amount / 10`。
- 只有 `Pending` 可取消，`Accepted` 后不可取消。
- 金额单位均为分，平台收费 10% 使用整数除法向下取整。

## 验证

```bash
cargo test
cargo check
```
