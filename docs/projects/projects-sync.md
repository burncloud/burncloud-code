# GitHub Projects V2 字段同步协议 v1.8

目标为组织项目 https://github.com/orgs/burncloud/projects/1 。**本文件不假定这些建议字段实际已存在**，必须先以 GraphQL Project Schema Discovery 获取真实 Project ID、Field ID、Select Option ID、字段类型和现有状态值。

## 建议字段（必须与现有人工流程兼容）
| 字段 | 类型 | 管理策略 |
|---|---|---|
| Status | single select | 只更新配置允许状态，尊重人工 Paused/Blocked |
| Priority | single select | 默认只读，除非明确授权写入 |
| Agent | text | BAM 受管字段（如存在） |
| PR | text | BAM 受管字段（如存在） |
| Latest HEAD | text | BAM 受管字段（如存在） |
| Blocked By | text | BAM 受管字段（如存在） |
| Merge Approval | single select | **不得把普通字段值当唯一合并授权凭证** |
| Last Activity / Last Error | date/text | 受管可选字段，节流写入 |
| Parent Issue | text | 展示层，权威关联仍为 GitHub Issue 原生关系 |

## 映射与冲突
`ProjectFieldMap` 包含 `logical_name, field_node_id, field_type, option_ids, managed, expected_value`。启动发现后按真实 Schema 校验，缺字段不能默认存在；bootstrap 先 dry-run，实际新增/改名需要用户确认。写入前重读远端，只提交白名单字段，不得覆盖人工无关字段。对同字段的人工覆盖要明确策略（冻结优先），冲突时停止写入而非反复覆盖。

## 增量轮询
60 秒默认 Poll，按 cursor 和更新时间水位同步 Issue/PR/Review/Checks/Project；每日完整对账。每次事件先写 SQLite inbox，再状态处理。一个 Project Item 的 `(project_node_id,content_node_id)` 应唯一识别；重复同步不得重复添加 Item。

## 现场发现结果模板
```text
Project Node ID: UNVERIFIED
Field logical name: Status
Field Node ID: UNVERIFIED
Field type: UNVERIFIED
Option IDs: UNVERIFIED
Managed by BAM: false until explicitly configured
Detected by: bam project inspect --json
```

## 安全及验收
PROJ-01 Schema 字段重命名停止错误写；02 其他人工字段不变；03 Paused Issue 不领取；04 重复扫描不重复 Item；05 无权限读取字段输出 UNKNOWN；06 审计可追踪每次字段变更；07 项目字段不能伪造 Merge Controller 授权。