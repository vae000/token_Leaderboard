# 架构说明

## 目录

- `apps/cli`: 员工本机采集器
- `apps/api`: 中央服务，负责认证、入库、聚合和查询
- `apps/web`: 榜单与个人面板
- `crates/common`: 共享的 API/领域模型
- `db/migrations`: PostgreSQL schema

## MVP 设计取舍

为尽快打通全链路，当前后端默认使用内存仓库承载数据与聚合计算，同时保留 SQLx 与 migration 接入点：

- 配置了 `API__DATABASE_URL` 时，服务启动时会创建 `PgPool` 并执行 migration
- 未配置数据库时，API 仍可直接提供演示数据和完整接口，方便 CLI/Web 联调

这让仓库在没有外部基础设施的情况下也能运行，同时不偏离计划中的正式架构。

## 聚合方式

- 原始事件进入仓库后会立刻参与日级聚合计算
- 仪表盘和榜单查询优先读取聚合结果视图，而不是逐页扫描明细
- 增长榜通过“当前窗口 vs 上一个等长窗口”计算

## CLI 数据流

1. `login` 获取设备 ID 与 token 骨架并持久化
2. `sync` 调用各 adapter 扫描本地日志
3. adapter 输出统一 `UsageEvent`
4. CLI 批量调用 `POST /v1/ingest/events:batch`
5. 成功后推进本地游标

## Web 数据流

- 页面采用 App Router 的服务端取数
- `lib/api.ts` 封装对后端 API 的调用
- 当后端不可达时，前端会回退到演示数据，保证页面可渲染
