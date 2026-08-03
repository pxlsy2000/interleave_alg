# Feature Backlog

更新时间：2026-08-03

本文只记录后续希望完成的功能、预期结果、待办项与待确认问题，不承诺排期、实现顺序或技术方案。条目顺序沿用需求提出顺序。

状态说明：

- `待办`：目标已提出，可以继续细化。
- `待决策`：继续实现前需要先确定产品或架构选择。
- `等待研究`：所需分析或资料尚未完成。

## BL-001：为 `run` 提供默认 HTML 可视化报告

**状态：待办**

**期望结果：** 用户执行 `interleave run` 后，默认获得可直接用浏览器查看的 HTML 报告，通过图标、颜色和图表直观看出不同 workload 下的性能表现；`text` 和 `json` 仍可按需选择，其中 JSON 继续作为稳定的机器可读输出。

- [ ] 将 `html` 加入 `run --format`，并将其设为 `run` 的默认人类可读格式。
- [ ] 明确未指定 `--output` 时 HTML 的落盘位置、文件命名、终端提示以及是否自动打开浏览器。
- [ ] 保留显式 `--format text` 与 `--format json`，明确三种格式的输出目标、覆盖、失败和大小限制行为。
- [ ] 让 HTML、text 和 JSON 对同一次分析表达相同结论，避免展示层重新计算指标。
- [ ] 在报告摘要中展示 Mapping 状态、warning、workload 数量和整体表现。
- [ ] 为每个 workload 展示访问数、各 Target 的 count/share、最大负载、短期窗口峰值和最长连续访问。
- [ ] 使用图标与一致的视觉语义区分通过、警告、失败、均衡、热点和异常结果，同时保留文字标签，不能只依赖颜色或图标传达含义。
- [ ] 提供适合比较不同 workload 的总览，并支持继续查看单个 workload 的详细指标。
- [ ] 生成自包含、离线可打开且不依赖外部 CDN 的 HTML 文件。
- [ ] 确保大量 Target、window 和 workload 下仍可阅读，并在桌面与窄屏浏览器中正常使用。
- [ ] 为 HTML 输出补充格式契约、示例、自动化测试和真实浏览器可视检查。

**待确认：**

- 默认 HTML 是否只适用于 `run`，还是也扩展到 `validate` 和 `map`。
- CLI 默认输出应生成文件、输出 HTML 到 stdout，还是生成文件后自动打开浏览器。
- 第一版图表使用纯 HTML/CSS/SVG，还是允许内嵌可视化库。

## BL-002：支持 DDR Row Switch / Address Mapping 评估

**状态：待决策**

**期望结果：** 对固定 NoC Mapping 和 workload，比较不同 MC `Bank / Row / Column` Address Mapping 引起的 Row Switch，并用 HTML 报告直观展示 Row Locality 与潜在 Bank-Level Parallelism。

第一版分析边界沿用现有研究中的简化模型：每个 Bank 独立维护 Open Row，按进入 MC 的请求顺序分析，不建模复杂 Scheduler、Refresh、读写切换、QoS 或完整 DDR Timing。该阶段评估 Mapping 本身造成的 Row 切换，不直接预测真实带宽。

### 架构归属

- [ ] 在实现前决定采用以下哪种产品形态：
  - 扩展现有 `interleave` 二进制，复用当前 Mapping、Scenario 和 `run` 流程；
  - 在当前仓库增加独立目录/模块和新的 DDR 分析二进制，共享必要的基础能力；
  - 仅在复用边界明显不足时再评估独立项目，避免过早拆分仓库。
- [ ] 用输入模型复用程度、概念边界、CLI 易用性、报告复用、版本演进和测试隔离作为决策依据。
- [ ] 记录最终选择及其理由，再移除本条目的 `待决策` 状态。

### 输入与正确性

- [ ] 定义 MC Address Mapping 输入，明确 `Bank`、`Row`、`Column` 分别使用 Local Address 的哪些 bit 或映射函数。
- [ ] 验证 Mapping 的地址唯一性、可逆性与容量匹配。
- [ ] 让全局地址先经过固定 NoC Mapping 得到 `(Target, Local Address)`，再按 Target/MC 分组，并保持每个 MC 实际看到的请求顺序。
- [ ] 支持顺序、不同 stride、block/tile、多 stream 交织和实际 trace 等 workload 来源；实际 trace 导入可作为独立子项，不与第一版强绑定。
- [ ] 为多个候选 MC Mapping 运行同一组输入，确保比较基准一致。

### 指标与选择规则

- [ ] 按 Bank 投影 Row 序列，统计每个 Bank 的 Row Transition 数 `T_b`。
- [ ] 统计全局 `N_conflict = sum(T_b)`，表示所有 Bank 的 Row Conflict 总数。
- [ ] 统计 `R_conflict = N_conflict / N_access`，用于比较不同长度的 trace 或 workload。
- [ ] 展示每个 Bank 的 `T_b` 与归一化冲突率，避免全局平均掩盖局部 Row Thrashing。
- [ ] 用滑动窗口计算 `BankSpread_K`、平均 BankSpread 和可配置的最低门槛 `B_min`。
- [ ] 使用“两层规则”比较候选方案：先排除平均 BankSpread 未达到 `B_min` 的 Mapping，再在合格候选中最小化 Row Conflict Rate。
- [ ] 多 workload 比较时支持显式权重，并对各 workload 的冲突率加权，避免长 trace 因访问数更多而自动占据更大权重。
- [ ] 当候选 Mapping 的 Row Conflict 与 BankSpread 非常接近时，仅提示需要更完整的 Timing/Queue/Scheduler 模型，不在第一版中伪造精确带宽结论。

### HTML 可视化

- [ ] 复用 BL-001 的 HTML 报告基础能力和视觉语义。
- [ ] 在总览中并列展示每个 Mapping 的 Row Conflict、Row Conflict Rate、平均 BankSpread 和约束是否通过。
- [ ] 直观标识最终候选：先显示 BankSpread 是否达标，再显示达标方案中的最低 Row Conflict。
- [ ] 按 Target、MC 和 Bank 下钻，展示 Row 序列变化、冲突热点和局部 Thrashing。
- [ ] 提供 Bank 访问分布与滑动窗口 BankSpread 视图，避免只展示一个全局平均值。
- [ ] 对每个图标、颜色和图表提供文字、数值或 tooltip 解释。

### 验证样例

- [ ] 固化研究资料中的 3-bit 顺序访问示例，验证 Mapping A/C 的冲突率为 25%，Mapping B 为 75%。
- [ ] 固化 stride-2 示例，验证较低 Row Conflict 可能同时伴随请求集中到单个 Bank。
- [ ] 覆盖空 Bank、单次访问、无 Row Transition、严重 Thrashing、窗口小于/等于 trace 长度边界和多 Target 场景。
- [ ] 为同一输入的 text/JSON/HTML 指标一致性增加验证。

**待确认：**

- 该能力应并入现有 `interleave`，还是作为同仓库的新二进制交付。
- 第一版 MC Mapping 只支持 bit field/位序，还是同时支持 XOR 等函数。
- `K` 与 `B_min` 使用显式配置、基于 Outstanding 深度的默认值，还是两者同时支持。
- 第一版是否需要真实 trace 导入；当前 v1 明确未包含该能力。

## BL-003：支持非 2 的整数次幂的 Target 数量

**状态：等待研究**

**期望结果：** 在保证 Mapping 正确性、结果可解释性和现有 2 的幂场景不回归的前提下，支持 Target 数量不是 2 的整数次幂的算法与评估。

- [ ] 等待并关联后续 Notion 分析页面。
- [ ] 基于研究结果明确算法、地址空间语义、Target 容量是否相等以及是否允许地址空洞。
- [ ] 重新定义 Target 可达、Mapping 唯一/可逆和 Local Address 语义，不能直接套用当前 `r = log2(N)` 的 GF(2) 矩阵条件。
- [ ] 明确非 2 的幂算法与当前 XOR Mapping 的关系：扩展现有 schema、增加新的 Mapping mode，或提供独立算法入口。
- [ ] 评估取模、拒绝区间、非均匀容量或其他候选方法对均衡性、局部性、硬件成本和可逆性的影响。
- [ ] 定义典型 workload、边界输入和与 2 的幂基线的比较指标。
- [ ] 明确 CLI、配置 schema、错误码以及 text/JSON/HTML 输出的兼容策略。
- [ ] 将当前 `mapping.unsupported` 的非 2 的幂路径转化为新能力时，保留清晰的版本与兼容性说明。
- [ ] 在研究结论确定后再拆分为可实现条目；在此之前不预设具体算法或目录结构。

**待确认：** 后续 Notion 资料的链接与分析结论。

## 共享事项

- [ ] 将可视化建立在稳定的内部报告数据上，使普通 `run` 与 DDR 分析共享样式、图标、可访问性和文件输出能力。
- [ ] 为新增报告定义确定性顺序、稳定数值格式、完整失败输出和原子文件写入行为。
- [ ] 保持中文与英文规格同步；backlog 本身暂以中文维护。
- [ ] 每个功能落地后更新 `docs/spec.md`、`docs/spec.en.md` 与 `tests/TRACEABILITY.md`，并从本文件勾选或移除已完成条目。

## 资料

- [当前中文规格](spec.md)
- [Current English specification](spec.en.md)
- [Notion：09.1 - DDR Mapping性能](https://app.notion.com/p/3aaa1f1a9ec481a0a484f1b56317c241)，读取于 2026-08-03；其中指标与分析边界最后编辑于 2026-07-28。
