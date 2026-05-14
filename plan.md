# Token Leaderboard 执行计划

## 1. 项目目标

构建一套“本地 AI 工具使用数据采集 + 中央聚合 + Web 榜单展示”系统，覆盖以下工具：

- Codex
- Cursor
- Claude Code
- OpenCode
- DeepSeek-TUI

系统分为 4 个子系统：

1. Rust CLI：安装在员工本机，负责登录、读取本地日志、标准化事件、批量上传。
2. Rust Backend：提供微信登录、账号绑定、上传接收、聚合任务、查询 API。
3. PostgreSQL：存储用户、团队、工具、模型、原始事件、日级聚合、排行榜快照。
4. Next.js Web：展示 Dashboard、排行榜、个人页。

首版目标是分钟级更新，即 CLI 周期性上传，后端按分钟或 5 分钟聚合，Web 读取聚合结果而不是直接扫原始日志。

## 2. 技术选型与工程结构

### 2.1 技术栈

- CLI：Rust
- Backend：Rust + Axum + Tokio + SQLx
- Web：Next.js App Router + TypeScript
- Database：PostgreSQL
- 配置与序列化：Serde

### 2.2 推荐目录结构

```text
apps/
  cli/          Rust CLI
  api/          Rust 后端
  web/          Next.js 前端
crates/
  common/       Rust 公共模型与工具
db/
  migrations/   PostgreSQL migration
docs/           接入文档与运维文档
```

### 2.3 系统边界

- CLI 负责本地采集、登录态保存、增量上传、基础脱敏。
- Backend 负责所有业务规则：身份绑定、团队归属、成本换算、聚合逻辑、排行计算。
- Web 只消费后端 API，不直接访问数据库。
- PostgreSQL 首版同时承载业务查询和聚合查询，不引入 Kafka、ClickHouse、Redis。

## 3. CLI 开发计划

### 3.1 CLI 功能目标

CLI 用于安装到用户本机后自动收集 AI 工具日志，并将数据上传到服务端。

### 3.2 CLI 命令设计

- `leaderboard login`
  - 打开微信登录页。
  - 完成 OAuth 后获取用户身份和设备绑定 token。
  - 本地保存 `refresh_token`、`device_id`、`upload_cursor`。
- `leaderboard sync`
  - 扫描支持工具的日志目录。
  - 解析新增日志并转换为统一事件。
  - 批量上传到后端。
  - 成功后推进本地游标。
- `leaderboard status`
  - 显示当前登录用户、上次同步时间、待上传事件数、已识别工具。
- `leaderboard logout`
  - 清理本地登录态与缓存。
- `leaderboard doctor`
  - 检查日志路径、权限、网络连通、登录状态。

### 3.3 CLI 模块拆分

- `cmd`
  - 各子命令入口。
- `auth`
  - 微信登录、token 保存、刷新、登出。
- `config`
  - 本地配置文件与状态持久化。
- `adapters`
  - 各工具日志采集适配器。
- `sync`
  - 增量读取、上传、游标推进。
- `http`
  - 与后端 API 通信。
- `diagnostics`
  - 环境自检。

### 3.4 日志采集设计

每个工具实现一个独立 adapter：

- `codex`
- `cursor`
- `claude_code`
- `opencode`
- `deepseek_tui`

每个 adapter 的职责：

- 发现日志路径。
- 增量读取日志。
- 提取时间、模型、输入 token、输出 token、请求数、会话 ID。
- 标记原始文件路径和偏移量。
- 输出统一结构。

统一事件结构 `UsageEvent` 建议字段：

- `event_id`
- `user_id`
- `tool`
- `model`
- `occurred_at`
- `input_tokens`
- `output_tokens`
- `cached_tokens`
- `estimated_cost_usd`
- `session_id`
- `source_file`
- `source_offset`
- `raw_hash`

### 3.5 上传协议

- 上传接口：`POST /v1/ingest/events:batch`
- 单批建议：500 到 2000 条
- 幂等键：
  - 优先 `raw_hash`
  - 备选 `tool + source_file + source_offset + timestamp`

后端返回：

- 接收条数
- 去重条数
- 拒绝条数
- 建议下次同步时间

### 3.6 CLI 实施顺序

1. 初始化 Rust CLI 工程。
2. 完成命令行框架。
3. 完成本地配置与状态存储。
4. 实现微信登录流程。
5. 实现 HTTP 客户端与上传协议。
6. 先接入一个工具 adapter，优先 `Codex` 或 `Cursor`。
7. 打通端到端采集与上传。
8. 扩展剩余工具 adapter。
9. 增加 `doctor` 与错误恢复能力。

## 4. Backend 开发计划

### 4.1 Backend 功能目标

后端负责认证、事件入库、聚合计算、排行榜查询以及管理能力。

### 4.2 Backend 模块拆分

- `auth`
  - 微信 OAuth
  - CLI token 签发、刷新、吊销
- `identity`
  - 用户主数据
  - 微信 openid/unionid 绑定
  - 设备绑定
  - 团队映射
- `ingest`
  - 接收批量事件
  - 校验 schema
  - 去重
  - 入库
- `aggregation`
  - 日级、周级、月级聚合
  - Dashboard 指标计算
  - 排行榜物化
- `query`
  - Dashboard、排行榜、个人页查询接口
- `admin`
  - 员工与团队导入
  - 工具开关
  - 模型成本配置

### 4.3 API 设计

认证接口：

- `POST /v1/auth/cli/start`
- `GET /v1/auth/cli/callback`
- `POST /v1/auth/cli/refresh`

采集接口：

- `POST /v1/ingest/events:batch`

查询接口：

- `GET /v1/dashboard/summary`
- `GET /v1/leaderboards/users`
- `GET /v1/leaderboards/teams`
- `GET /v1/leaderboards/tools`
- `GET /v1/leaderboards/models`
- `GET /v1/leaderboards/growth`
- `GET /v1/me/overview`
- `GET /v1/me/trend`
- `GET /v1/me/distribution`
- `GET /v1/me/rewards`

管理接口：

- `POST /v1/admin/team-memberships:import`
- `PUT /v1/admin/users/:id/team`

### 4.4 聚合与统计口径

- 全公司总 token：累计输入 token + 输出 token
- 本周活跃用户：最近 7 天有事件上报的去重用户数
- 本月成本：按事件发生时对应模型价格汇总
- AI 渗透率：最近 30 天有 AI 使用记录的员工数 / 已导入员工总数
- TOP 工具：按 token 总量排序
- TOP 模型：按 token 总量排序
- 个人榜：按周期总 token 排序
- 团队榜：按团队总 token 排序，并附带人均 token 字段
- 工具榜：按工具总 token 排序
- 模型榜：按模型总 token 排序
- 增长榜：本周期相较上周期增量排序

### 4.5 聚合实现方式

- 原始事件写入 `usage_events`
- 聚合表按天存储：
  - `usage_agg_user_day`
  - `usage_agg_team_day`
  - `usage_agg_tool_day`
  - `usage_agg_model_day`
- 聚合任务按分钟或 5 分钟执行
- 榜单查询优先读聚合表
- 聚合任务必须支持幂等重跑与时间窗口增量刷新

### 4.6 Backend 实施顺序

1. 初始化 Rust API 工程。
2. 建立配置系统、日志系统、数据库连接。
3. 实现健康检查接口。
4. 完成微信登录与 CLI token 骨架。
5. 实现事件 ingest 接口。
6. 完成原始事件入库与去重。
7. 完成团队导入与用户绑定逻辑。
8. 实现日级聚合任务。
9. 实现 Dashboard API。
10. 实现排行榜 API。
11. 实现个人页 API。
12. 增加管理接口与配置能力。

## 5. PostgreSQL 数据库计划

### 5.1 核心表设计

- `users`
  - 员工基础信息
  - 微信标识
  - 状态
- `teams`
  - 团队树
  - 上级团队
  - 展示名称
- `user_team_memberships`
  - 用户与团队映射
  - 生效时间
- `devices`
  - CLI 安装设备与登录态
- `tool_types`
  - 工具定义
- `model_catalog`
  - 模型名称
  - 厂商
  - 单价
  - 计费单位
  - 生效时间
- `usage_events`
  - 原始事件明细
  - 幂等键
  - 解析来源
- `usage_agg_user_day`
- `usage_agg_team_day`
- `usage_agg_tool_day`
- `usage_agg_model_day`
- `reward_rules`
  - 奖励规则配置
- `reward_results`
  - 用户奖励结果快照

### 5.2 关键约束

- `users.wechat_openid` 唯一
- `users.wechat_unionid` 唯一
- 用户归并优先使用 `unionid`
- `usage_events.idempotency_key` 唯一
- 聚合表按维度 + 日期唯一
- 模型价格配置按 `model + effective_from` 版本化
- 所有时间统一存 UTC

### 5.3 索引建议

- `usage_events`
  - `occurred_at`
  - `user_id`
  - `tool`
  - `model`
  - `idempotency_key`
- 聚合表
  - `date`
  - `team_id`
  - `user_id`
  - `tool`
  - `model`

### 5.4 数据库实施顺序

1. 编写基础 migration 框架。
2. 创建主数据表。
3. 创建原始事件表。
4. 创建聚合表。
5. 创建奖励相关表。
6. 补齐唯一约束、外键与索引。
7. 准备初始化数据脚本：
  - 工具列表
  - 模型价格表

## 6. Web 开发计划

### 6.1 页面范围

#### Dashboard

- 全公司总 token
- 本周活跃用户
- 本月成本
- AI 渗透率
- TOP 工具
- TOP 模型
- 最近 30 天 token 趋势

#### 排行榜

- 个人榜
- 团队榜
- 工具榜
- 模型榜
- 增长榜

支持：

- 今日
- 本周
- 本月
- 自定义区间
- 团队筛选
- 工具筛选
- 模型筛选

#### 个人页

- 我的 token
- 我的工具分布
- 我的模型分布
- 我的趋势
- 我的奖励

### 6.2 Web 技术实现原则

- 使用 Next.js App Router
- 以服务端取数为主
- Dashboard 与排行榜页面采用 SSR 或 ISR
- 图表组件只接收后端整理后的数据
- 登录统一走微信 OAuth，由后端换发 session/cookie

### 6.3 Web 模块拆分

- `app/(dashboard)`
  - 首页 Dashboard
- `app/leaderboards`
  - 排行榜页面
- `app/me`
  - 个人页
- `components`
  - 指标卡片
  - 图表组件
  - 榜单表格
- `lib/api`
  - 服务端请求封装
- `lib/auth`
  - 登录态处理

### 6.4 Web 实施顺序

1. 初始化 Next.js 项目。
2. 接入基础布局、路由与鉴权。
3. 开发 Dashboard 页面。
4. 开发排行榜页面。
5. 开发个人页。
6. 增加筛选、时间范围切换与空态处理。
7. 完成登录跳转与受保护页面控制。

## 7. 奖励系统首版计划

奖励系统首版只做展示，不对接真实发奖。

实现内容：

- 后台可配置奖励规则。
- 聚合任务按规则生成奖励结果快照。
- 个人页展示“我的奖励”。

首版规则建议：

- 月度 token TOP N
- 工具使用覆盖度奖励
- 增长最快奖励

## 8. 开发阶段划分

### 阶段一：基础骨架

- 建立 monorepo
- 初始化 CLI、API、Web
- 建立数据库 migration
- 建立 CI

### 阶段二：采集链路打通

- 微信登录
- CLI 上传
- 后端接收与入库
- 接入首个工具 adapter

### 阶段三：聚合与 Dashboard

- 日级聚合
- 成本计算
- Dashboard API
- Dashboard 页面

### 阶段四：排行榜与个人页

- 各类榜单 API
- 个人页 API
- Web 排行榜与个人页

### 阶段五：补齐能力

- 剩余工具 adapter
- 奖励系统
- 管理接口
- 部署文档与监控

## 9. 测试计划

### 9.1 CLI 测试

- 登录成功、刷新 token、退出登录
- 本地无日志文件
- 无日志权限
- 日志损坏
- 网络失败自动重试
- 多次 `sync` 不重复上传
- 各 adapter 对真实日志样例解析正确

### 9.2 Backend 测试

- 微信 OAuth 成功与失败场景
- 重复绑定处理
- ingest schema 校验
- 幂等去重
- 批量部分失败返回
- 聚合任务重复执行结果一致
- 成本计算按历史价格版本正确回放
- 团队变更后历史数据不串团队

### 9.3 Database 测试

- migration 可重复执行
- 关键查询索引生效
- 百万级事件下聚合性能可接受

### 9.4 Web 测试

- Dashboard 首屏渲染成功
- 各榜单排序正确
- 时间筛选正确
- 空数据场景展示正常
- 个人页趋势与分布正确
- 登录过期后的跳转与拦截正常

### 9.5 联调验收

- 一名用户从 `login -> sync -> Web 可见个人数据` 全链路打通
- 多名用户、多团队数据进入后，Dashboard 和排行榜在 5 分钟内刷新
- 任一工具日志新增后，下一轮同步可正确入库并体现到榜单

## 10. 默认假设

- 首版为公司内部单租户系统
- 微信只负责身份认证，不提供组织架构
- 团队归属首版由后台维护的员工-团队映射表提供
- 数据时效目标为 1 到 5 分钟
- 成本字段先按静态模型价格表估算
- 奖励规则首版只展示，不接真实发奖系统
- Web 与 Backend 独立部署
- CLI 和 Backend 使用 Rust，Web 使用 Next.js，数据库使用 PostgreSQL
