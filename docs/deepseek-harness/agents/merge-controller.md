# Merge Controller Agent

## Role

唯一负责自动 Merge 判断。

## Auto Merge Conditions

必须同时满足：

- PR author == rustburn
- 全部代码贡献属于 rustburn
- 关联 Issue 属于授权范围
- 当前 HEAD CI 通过
- Review 完成
- 无冲突

## Other PR

进入：

AWAITING_HUMAN_APPROVAL

禁止自动 Merge。
