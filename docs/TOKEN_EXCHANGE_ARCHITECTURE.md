# AI Singularity - 去中心化 AI Token 交易所架构设计蓝图 (Deep Dive)

> **文档状态**: Architecture Specification v2.0
> **更新日期**: 2026-04-08
> **核心定位**: 一个自由交易、按量付费的 AI 去中心化路由分发池 (Free-Market Decentralized API Routing Exchange)。

本文档在基础愿景之上，深入阐述系统拓扑、核心算法机制、核心表结构设计以及对抗性防刷风控体系，为下一阶段的开发提供具体的工程化指导。

---

## 1. 核心系统拓扑 (System Topology)

整个交易所网络分为三大物理集群：**中央路由枢纽 (The Hub)**、**边缘节点网络 (The Miner Edge)** 和 **消费端应用 (The Consumers)**。

```mermaid
graph TD
    %% 需求侧
    C1[Consumer API Client A] -->|HTTP POST /v1/chat/completions\nToken: Strategy-Econ| GW
    C2[Consumer API Client B] -->|HTTP POST\nToken: Strategy-Premium| GW

    %% 中央处理核心
    subgraph 中央路由枢纽 (Central Hub)
        GW[API 网关层 (Axum/Rust)\n鉴权防刷 / SSL 卸载]
        RB[撮合调度引擎 (Router/Bidding)]
        LR[账本中心 (Ledger)\nRedis + Lua 原子扣费]
        HE[探针与信誉雷达\n(Health & Reputation Engine)]

        GW --> RB
        GW --> LR
        RB <--> HE
    end

    %% 供应侧 (去中心化网络)
    subgraph 边缘供应网络 (Miner Edge Network)
        M1[Tauri 本地节点 A\n(拥有 GPT-4 额度)]
        M2[Tauri 本地节点 B\n(运行 Ollama Llama3)]
        M3[服务器节点 C\n(拥有 Claude 3 灰产 API)]
    end

    %% P2P 长连接
    RB <==>|WebSocket 穿透长连\n(或 gRPC over HTTP2)| M1
    RB <==>|双向心跳与指令派发| M2
    RB <==>|指令派发| M3

    %% 数据持久层
    DB[(PostgreSQL / 关系型主库)]
    LR -.异检查异步落库.-> DB
    HE -.信誉快照落库.-> DB
```

---

## 2. 深度剖析：核心调度算法与信誉系统 (Smart Routing & Reputation)

传统的 API 网关是轮询 (Round Robin) 或随机分发。而作为交易所，核心资产是**算法**。我们采用 **基于信誉与价格出价的多维加权算法 (Price-Reputation Weighted Routing)**。

### 2.1 流量撮合流程
当一个消费端请求进入大盘，指定了价格策略策略池（例：`max_price: 1.0 ¥/1M`）：
1. **资产快照 (Snap)**：路由引擎从内存 (Redis Hash) 瞬间拉出所有单价 `<= 1.0` 且标记为 `Online` 的闲置节点。
2. **权重打分 (Scoring)**：公式：`Score = (Base_Reputation * 0.7) + (1 / Latency_ms * 0.3)`
3. **分发 (Dispatch)**：请求投递给 Score 最高的闲置节点。若该节点在 500ms 内未产生 TCP 响应，记录一次 `Timeout Penalty`，并立刻转移给次高分节点。

### 2.2 节点信誉雷达 (Reputation Radar Engine)
信誉分机制是维持“无平台人工客服干预”的核心。分数区间 `0 - 1000`。
*   **初始上架**：新节点给予 `500` 观察分。只能接低频/低价“冷发启动单”。
*   **正向奖励机制 (+分数)**：
    *   连续稳定完成 100 次 `Streaming` 输出无断连：+5 分。
    *   TTFT (首字生成时间, Time-to-first-token) 在池中排名前 10%：由于提供了超出当前价格的优质体验，额外 +2 分。
*   **严厉惩处机制 (-分数/Slash)**：
    *   TCP 握手拒绝 / 掉线未报备：-10 分（扣留当前进行中的未结订单资金）。
    *   返回 `401 Unauthorized` 或 `429 Too Many Requests`：系统证实该节点的 Key 已经死亡或限流，强行令节点下线（休眠锁），并扣除 50 分。
*   **淘汰线**：信誉低于 `100` 的节点不再分发任何订单，矿工余额冻结 3 天以防欺诈。

---

## 3. 深度剖析：极速微支付计算层 (Micro-Payment Ledger)

AI API 计费由于金额极小且并发极高，绝不能每次请求操作主数据库（如 MySQL/PostgreSQL）。必须由驻留在内存的原子运算接管。

### 3.1 毫秒级异步账本
*   **预扣控制 (Pre-auth)**：消费者发送请求瞬间，通过 Redis 执行一段极简 Lua 脚本：检查余额是否大于本次调用的最高理论值 (Max Tokens)，满足则放行，否则极速返回 402 Payment Required。
*   **流式原子扣账 (Stream-Delta Deduct)**：
    *   Rust 引擎作为透明代理转发 Stream Chunk。
    *   内置的高效 Tokenizer（如 `tiktoken-rs`）在内存中实时计算已过去的数据包。
    *   流结束时（收到 `[DONE]`），在单点触发 Redis 更新。此时才会把真正的 Token 数乘以该笔订单的“成单单价”进行扣费，剩下的闲置预授权额度释放。
*   **对账结算中心**：每日凌晨，后台的 Cron Job 会将一天内在 Redis 产生的全部变动结算到 PostgreSQL，清洗脏账，形成永久流水，此时再切分平台的手续费抽成，将数字划拨给旷工节点余额。

### 3.2 应对流式中断的资金保护防线
如果在打流过程中（比如已经输出了 50%），**底层矿机网线被拔**：
*   **资金协议**：由于未拿到结束标识 `[DONE]`，交易所视本次交易为技术事故 (Torn-Transaction)。
*   处理：**本次消耗的余额对消费者全额退回**(容错体验至上)；对该当事矿机**取消收益发放**并扣除信誉分。

---

## 4. 安全风控与反作弊 (Anti-Fraud & Trust & Safety)

在一个零准入的去中心化网络中，“作恶成本”极低。风控逻辑优先级远高于普通功能。

### 4.1 节点伪造假数据骗费（虚假请求骗局）
**欺诈场景**：矿工并没有把请求发给 OpenAI，而是用一个本地的脚本当收到请求时，疯狂往回吐大量毫无意义的垃圾文字流（“哈哈哈哈哈...”），企图骗取高额 Token 费。
**平台应对（金丝雀探针 - Golden Prompt）**：
*   系统会按千分之一的概率向各个节点掺杂一条“金丝雀请求”。
*   比如系统伪装成普通消费者问：“1+1等于几，只能回数字”。
*   如果节点的回复流超出了预定规则区间，或者长篇大论试图骗 Token，系统瞬间捕捉到！
*   **惩罚（Slashing）**：直接判定作弊，封毁该节点账户下的所有财产（永久拉黑），没收的资金放入平台的赔付准备资金池。

### 4.2 隐私审查与节点偷看数据
**隐私场景**：矿工偷偷记录（Log）消费方发过来的隐私业务提示词（Prompt/Company Data）。
**平台应对**：
*   在 Web 2.5 和常规 API 体系下，**完全杜绝恶意物理节点偷看明文数据是不可能的**，除非使用 TEE（可信执行环境机密计算），但这并非普通用户的设备能达到的。
*   **策略 1 (声明式契约)**：对于高敏感需求端，提供“官方直营池 / 顶级白名单池”，由平台官方的实名大企业身份背书。
*   **策略 2 (协议级惩罚)**：桌面客户端由我们的 Rust 源码编译，核心转发逻辑做二进制加壳或代码混淆。禁止第三方重写矿工客户端。探测到客户端 API 心跳指纹不符即封杀。

---

## 5. 核心持久化模型概览 (Core Schema Design Concept)

为了支撑大盘，我们初期需要规划如下实体蓝图：

*   **表 1: `users` (统筹者)**：
    *   身份既可以是开发者消费者，也可以是挂机矿工。
    *   字段：`id`, `wallet_address` / `balance_fiat` (双币结算余额), `status`。
*   **表 2: `miner_nodes` (矿工节点池)**：
    *   记录全网边缘电脑。
    *   字段：`node_id`, `owner_user_id`, `supported_models` (数组), `price_multiplier` (自行设置的比率), `reputation_score` (极其核心的信誉分), `is_online_now`。
*   **表 3: `routing_tiers` (消费者策略金牌)**：
    *   供开发者选购的大盘策略凭证（也就是他们请求时用的 API Key）。
    *   字段：`api_key_hash`, `owner_user_id`, `tier_type` (枚举：捡漏档/均衡档/极速直营档), `max_price_threshold` (限价单阈值)。
*   **日志/时序库: `transaction_ledgers` (海量计费表)**：
    *   这是数据量最庞大的交易纪实。建议放入 ClickHouse 或只保留按日汇总。
    *   字段：`txn_id`, `consumer_key_id`, `miner_node_id`, `model`, `prompt_tokens`, `completion_tokens`, `total_fee`, `platform_cut`, `create_time`。

---

## 6. 指导接下来的开发路线 (Next Actions)

这个极其精细和复杂的系统无法通过一轮迭代完成。对应到我们代码仓库当下的 `AI Singularity`：

1.  **首要目标**：当前，最紧迫的是**构建完全解耦的单机网关引擎架构**。所有的架构必须脱离“硬编码的单个 API Key 调用”。必须建立一个内部的“Provider 池”，在处理请求时体会“根据算法动态挑选 Provider 并转发”。
2.  **次级目标**：引入 `tiktoken` 相关库，建立一个独立的、高度内聚的**计费计算服务进程**，跑通准确的 “流式转发 -> Token 数量精准校验” 闭环。

如果我们能把“本地十几个 Key 的智能轮询和精准计时计费”完美做出来，我们就已经成功跨入了交易所大门的 50%！
