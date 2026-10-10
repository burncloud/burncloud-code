# BAM 验收与 CI 证据 Schema

## 证据来源
必须引用 GitHub 实际对象：PR Head SHA、Required Policy 版本/来源、Workflow Run ID、Check Run ID、Job/Step 结果、测试执行报告、Review 与 Mergeability 状态。Agent 报告仅可作为非权威说明。

## JSON Contract v1
```json
{
  "schema_version":"1.0",
  "task_id":"uuid",
  "issue":{"repository_id":0,"number":0,"node_id":"I_..."},
  "pr":{"repository_id":0,"number":0,"node_id":"PR_...","head_sha":"40-char-full-sha"},
  "base_sha":"40-char-full-sha",
  "merge_ref_sha":null,
  "required_policy":{"source":"github_rulesets","verified":false,"version":null},
  "checks":[{"context":"Cargo Test","app":null,"run_id":null,"job_id":null,"check_run_id":null,"sha":"40-char-full-sha","status":"completed","conclusion":"unknown","actually_executed":false,"url":null}],
  "criteria":[{"id":"M08-01","test_ids":["T-008"],"verdict":"UNKNOWN","evidence_ids":[]}],
  "review":{"verdict":"UNKNOWN","blocking_reviews":[]},
  "mergeability":"UNKNOWN",
  "local_compiles":0,
  "local_tests":0,
  "verdict":"UNKNOWN",
  "reason_codes":["MISSING_REQUIRED_EVIDENCE"],
  "evaluated_at":"RFC3339"
}
```

上述值为**Schema 说明用假数据**，不代表任何真实 CI。实际数据需检查类型合法性，禁止使用字符串 `"success"` 在没有 run/check ID、SHA、运行证明时冒充 PASS。

## Verdict 规则
`PASS`：所有必需门禁、Criteria、Review、Mergeability 全部获得匹配当前 HEAD 的正面证据。`FAIL`：有可信失败结果；`BLOCKED`：权限/授权/依赖/环境阻断；`UNKNOWN`：信息不足或远端检查异常。任何非 PASS 不能晋级到 Ready for Review。

## 失效条件
新 HEAD、base/ruleset 变化、Review Requested Changes、Check 作废、发现 Required Job SKIPPED、实际测试路径遗漏、数据源不可读 → 证据需重新采集。发布前再次 fetch GitHub，比较 HEAD；不一致整份报告废弃。

## 留存与脱敏
日志可以保留摘要和回链，敏感 token/raw secret 必须脱敏；对每个判定保留 SHA、timestamp、policy hash、source URL、check identity，保证可审计。CI 日志过期或证据 URL 失效不能继续宣称可重新验证通过。