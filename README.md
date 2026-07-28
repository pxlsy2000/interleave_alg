# interleave_alg

`interleave_alg` 是一个面向 Hash Interleave 设计、验证和性能分析的工具项目。它用于判断地址 Mapping 是否正确，并分析典型访问模式下的 Target 均衡性和短时拥塞。

## 当前状态

项目目前处于规格阶段，CLI 和分析功能尚未实现。

[完整产品规格](docs/spec.md)是本项目行为定义的唯一事实源。它规定数学模型、正确性标准、YAML 输入、命令行接口、性能指标、输出格式、错误行为和 corner case。

## 计划提供的能力

- 验证所有 Target 是否可达；
- 验证 `(Target, Local Address)` Mapping 是否为双射；
- 识别每个 Target 内的 Local Address 是否自然有序；
- 查询具体 byte address 的 Target 和 Local Address；
- 描述 stride、sweep 和 multi-stream 访问场景；
- 分析长期负载、滑动窗口拥塞和最长连续同 Target 访问；
- 生成人类可读报告和结构化 JSON 报告。

## 文档导航

| 文档 | 面向对象 | 职责 |
| --- | --- | --- |
| [README](README.md) | 工具使用者 | 介绍项目、当前状态和已经实现的使用方式 |
| [产品规格](docs/spec.md) | 使用者、设计者、实现者 | 定义产品行为和验收标准，是实现与评审的最终依据 |
| [实现计划](docs/plans/README.md) | 开发者 | 将产品规格拆成可执行的开发步骤，记录计划约定 |

## 文档维护原则

- 产品行为发生变化时，先修改产品规格；
- 实现计划必须引用产品规格，不能另行定义产品行为；
- README 只描述已经实现并验证过的能力；
- README 与实现不一致时，应修正 README；实现与产品规格不一致时，应修正实现或先明确修改规格。

实现完成后，本 README 将补充安装、快速开始、配置示例和常见工作流。
