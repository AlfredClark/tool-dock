// CodeMirror 语言包映射：JSON/XML 用官方包，其余经 legacy-modes 复用。
// INI 无独立模式，与 Properties 共用 properties 模式；调用方传 null 即纯文本。
import { json } from "@codemirror/lang-json";
import { xml } from "@codemirror/lang-xml";
import { StreamLanguage } from "@codemirror/language";
import { properties } from "@codemirror/legacy-modes/mode/properties";
import { toml } from "@codemirror/legacy-modes/mode/toml";
import { yaml } from "@codemirror/legacy-modes/mode/yaml";
import type { Extension } from "@codemirror/state";
import type { ConvertFormat } from "./formats";

/** 各格式的高亮工厂：Record 键覆盖全部格式，增减格式时编译期拦截 */
const EXTENSION_FACTORIES: Record<ConvertFormat, () => Extension> = {
  json: () => json(),
  yaml: () => StreamLanguage.define(yaml),
  xml: () => xml(),
  toml: () => StreamLanguage.define(toml),
  ini: () => StreamLanguage.define(properties),
  properties: () => StreamLanguage.define(properties),
};

/** 取格式对应的高亮扩展（每次返回新实例，供 Compartment 重配） */
export function extensionForFormat(format: ConvertFormat): Extension {
  return EXTENSION_FACTORIES[format]();
}
