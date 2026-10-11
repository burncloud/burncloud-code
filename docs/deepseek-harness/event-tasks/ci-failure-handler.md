# BurnCloud CI失败事件参考（未启用）

## 与原生定时任务的关系

当前交付仅配置01至09的DeepSeek Harness原生定时任务。05每10分钟查询GitHub Actions，在PR普通留言发布CI_RESULT；03每10分钟读取当前有效失败并修复。没有Webhook也能完成此闭环。

本文件只保留未来另行明确授权的事件加速参考，不是第10或第11个注册任务，不要求接收服务、队列、worker池或外部DB。当前文档PR不创建Webhook、不修改Actions触发器或CI。

## 可选能力边界

若未来已有明确授权及验证可用的事件接收能力，外部事件只作为05提前查询真实CI的提示，不能构成另一条跨任务通信通道，不能直接调用03、向私有邮箱派发或另发from=05协议消息。

接收端须依据GitHub官方签名协议核验原始请求和仓库，失败则拒绝。delivery只标识投递，不能证明任务授权、Issue作者或贡献者。payload的run/attempt/HEAD/PR也须API重读。接收能力不存在时保持“未启用”，05定时轮询仍正常工作。

05依照同一[留言协议](../communication-protocol.md)查询当前HEAD所有适用检查、核验最新attempt，发布唯一业务结果，原生互斥及去重不因事件入口获得豁免。事件重复、遗漏或旧HEAD迟到不能重置预算、重复修复或覆盖当前成功。

## 故障分类

| 实际原因 | 定时路径动作 |
|---|---|
| 源码/测试失败 | 05在PR发布CI_RESULT；03下轮核验后修复源码/测试 |
| 临时Runner/网络故障 | 05在已有权限内最多重跑一次并发布CI_RETRY_RESULT |
| 覆盖缺口、权限或持久基础设施故障 | 05记录阻塞；08按11职责记日志；不修改CI |
| 取消、超时、跳过、需操作 | 05读真实日志诊断；未知保持未通过 |
| 旧HEAD/旧attempt、关闭PR、非白名单 | 不开发、不重跑、不合并；外部对象只读忽略 |

所有修复预算跨HEAD累计，代码开发、CI重跑、合并仍分别归03、05、06。事件成功接收不是修复完成，更不是Review或合并批准。

## 官方参考

- [GitHub Webhook事件](https://docs.github.com/en/webhooks/webhook-events-and-payloads#workflow_run)
- [GitHub Webhook签名验证](https://docs.github.com/en/webhooks/using-webhooks/validating-webhook-deliveries)
