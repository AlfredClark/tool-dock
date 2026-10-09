// 重命名器工具的界面态类型：后端真实改名未接入，规则与预览均为前端自有状态。
// 规则只作用于主名（去扩展部分），扩展名默认保留。

/** 排序键：第一步仅支持按名称升/降序 */
export type RenamerSortKey = "name-asc" | "name-desc";

/** 文件列表项：`checked` 为行复选框选中态（删除选中的依据） */
export interface RenamerFileItem {
  /** 稳定键：桌面端用完整路径，浏览器用合成键（改名成功后同步换为新路径） */
  id: string;
  /** 本地路径（浏览器降级为空串，不可执行改名） */
  path: string;
  /** 显示名：路径取 basename，浏览器取 File 名 */
  name: string;
  /** 新文件名：规则派生预览，无规则时恒等于 `name` */
  newName: string;
  /** 行复选框选中态 */
  checked: boolean;
  /** 执行结果：`short` 行徽章展示，`detail` 为悬浮诊断（成功/未执行为 `null`） */
  error: RenamerFileError | null;
}

/** 行执行结果：跳过原因或失败诊断常驻行内（toast 消失后仍可查看） */
export interface RenamerFileError {
  /** 徽章短文本（已映射通用文案） */
  short: string;
  /** 悬浮英文诊断原文 */
  detail: string;
}

/** 规则类型：添加下拉框的七个候选项 */
export type RenamerRuleKind =
  "affix" | "strip" | "case" | "replace" | "regex" | "number" | "normalize";

/** 规则公共字段：稳定键 + 启用开关（禁用规则跳过不参与预览） */
interface RenamerRuleBase {
  /** 稳定键：`createRule` 内 `crypto.randomUUID()` 生成 */
  id: string;
  /** 启用开关：`false` 即预览跳过 */
  enabled: boolean;
}

/** 前/后缀位置 */
export type RenamerEdge = "prefix" | "suffix";

/** 大小写模式 */
export type RenamerCaseMode = "upper" | "lower" | "title";

/** 改名规则：判别联合体，`kind` 收窄后读写各自参数 */
export type RenamerRule =
  | (RenamerRuleBase & { kind: "affix"; position: RenamerEdge; text: string })
  | (RenamerRuleBase & { kind: "strip"; position: RenamerEdge; count: string })
  | (RenamerRuleBase & { kind: "case"; mode: RenamerCaseMode })
  | (RenamerRuleBase & {
      kind: "replace";
      find: string;
      replacement: string;
      matchCase: boolean;
    })
  | (RenamerRuleBase & { kind: "regex"; pattern: string; replacement: string })
  | (RenamerRuleBase & {
      kind: "number";
      start: string;
      step: string;
      digits: string;
      position: RenamerEdge;
      separator: string;
    })
  | (RenamerRuleBase & { kind: "normalize" });
