# Trading Platform

一个基于 Axum 和 PostgreSQL 的订单流转示例项目，覆盖需求方创建订单、服务方接单、需求方确认完成、资金冻结和平台手续费记录。

架构设计：项目按 `router -> handler -> service -> policy/repository -> model` 分层，HTTP 入口、业务编排、规则校验和数据库访问分别放在独立模块中。

## 功能范围

当前实现两个订单接口：

- `POST /orders`：创建需求订单。
- `PATCH /orders/{id}/status`：更新订单状态。

订单状态流转：

```text
Pending -> Accepted
Pending -> Cancelled
Accepted -> Completed
```

资金规则：

- 金额单位为分。
- 创建订单时校验需求方余额，但不冻结资金。
- `Pending -> Accepted` 时冻结需求方可用余额。
- `Accepted -> Completed` 时扣减冻结余额，服务方入账，平台记录 10% 手续费。
- 手续费使用整数除法向下取整。

## 技术栈

- Axum
- SQLx
- PostgreSQL

## 项目结构

```text
db/                      # 数据库初始化脚本
migrations/              # 迁移脚本
src/
├── common/              # 统一响应结构
├── config.rs            # 运行期配置
├── errors/              # 应用错误类型和响应转换
├── modules/
│   ├── orders/          # 订单模块
│   │   ├── handler.rs   # HTTP 请求处理
│   │   ├── service.rs   # 业务编排和事务
│   │   ├── policy.rs    # 业务规则
│   │   ├── repository.rs# 订单表访问
│   │   └── model/       # DTO、实体和值类型
│   └── users/           # 用户模型和用户表访问
├── router/              # 总路由组合
├── lib.rs
└── main.rs
```

## 本地运行

### 1. 准备 PostgreSQL

Docker  启动 PostgreSQL：
```bash
# 终端执行
docker run --name trading-postgres -e POSTGRES_USER=postgres -e POSTGRES_PASSWORD=postgres -p 5432:5432 -d postgres:16
```
**用户名和密码默认为 `postgres`**

### 2. 配置环境变量

在项目根目录创建 `.env`(复制 `.env.example` 内容然后修改即可)：

```env
# 示例中：postgres:postgres 分别的代表用户名和密码
DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres_db
DATABASE_MAX_CONNECTIONS=5
APP_HOST=127.0.0.1
APP_PORT=3000
LOG_LEVEL=debug
```

配置说明：

| 变量 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- |
| `DATABASE_URL` | 是 | 无 | PostgreSQL 连接字符串 |
| `DATABASE_MAX_CONNECTIONS` | 否 | `5` | 数据库连接池最大连接数 |
| `APP_HOST` | 否 | `127.0.0.1` | HTTP 监听地址 |
| `APP_PORT` | 否 | `3000` | HTTP 监听端口 |
| `LOG_LEVEL` | 否 | `debug` | `trace`、`debug`、`info`、`warn`、`error` |

### 3. 初始化数据库
#### 方案1：容器内执行 sql 脚本
+ 创建数据库
```bash
# 项目根目录下执行
docker exec -it trading-postgres createdb -U postgres postgres_db
# 拷贝脚本文件到容器
docker cp .\db\init.sql trading-postgres:/tmp/init.sql
# 执行脚本生成库表及样例数据
docker exec -it trading-postgres psql -U postgres -d postgres_db -f /tmp/init.sql
```

#### 方案2：使用 SQLx 迁移（推荐）
+ 安装 sqlx-cli
```bash
cargo install sqlx-cli
```

+ 创建数据库
```bash
# 项目根目录下执行
sqlx database create
```

+ 执行迁移文件
```bash
# 项目根目录下执行
sqlx migrate run
```

初始化脚本会创建：

- `users`
- `orders`
- `platform_fee_records`
- `_sqlx_migrations` (如果使用方案2，会自动创建该表)

并插入示例用户和订单数据(注意，请完成以上初始化数据库操作后再使用图形界面工具访问，否则时区显示有误)。

### 4. 启动服务

```bash
# 项目根目录下执行
cargo run
```

默认监听：

```text
http://127.0.0.1:3000
```

### 5. 健康检查

```bash
curl http://127.0.0.1:3000/health
```

## API

### 创建订单

```http
POST /orders
```

请求体：

```json
{
  "client_id": 1,
  "worker_id": 3,
  "amount": 10000,
  "description": "预约一次服务"
}
```

示例：

```bash
curl -X POST http://127.0.0.1:3000/orders \
  -H "Content-Type: application/json" \
  -d '{"client_id":1,"worker_id":3,"amount":10000,"description":"预约一次服务"}'
```

### 更新订单状态

```http
PATCH /orders/{id}/status
```

请求体：

```json
{
  "target_status": "Accepted",
  "operator_id": 3
}
```

接单：

```bash
curl -X PATCH http://127.0.0.1:3000/orders/1/status \
  -H "Content-Type: application/json" \
  -d '{"target_status":"Accepted","operator_id":3}'
```

确认完成：

```bash
curl -X PATCH http://127.0.0.1:3000/orders/1/status \
  -H "Content-Type: application/json" \
  -d '{"target_status":"Completed","operator_id":1}'
```

取消订单：

```bash
curl -X PATCH http://127.0.0.1:3000/orders/1/status \
  -H "Content-Type: application/json" \
  -d '{"target_status":"Cancelled","operator_id":1}'
```

## 生产环境需要额外考虑
+ 创建订单规则过宽，只要余额足够就能创建任意订单，可能出现刷单、恶意指定服务方
+ 真实的认证授权，用户的 ID 等信息需要从后端获取，不能靠前端传入
+ 订单相关接口需要考虑幂等性，防止重复重建订单或重复结算
+ 接口需要做限流策略，防止恶意请求压垮服务
+ 当前只更新余额和记录手续费，缺少可审计的账本，可以增加账本表，每次冻结、解冻、扣款、入账、手续费都写入双向流水
+ 需要考虑结算异常情况时的补偿机制
+ 预防并发与死锁问题

