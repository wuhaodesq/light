# 对话语言要求 / Language Requirement

**所有对话必须使用中文进行交流。**

All conversations must be conducted in Chinese.

---

# Light Language 开发进度 / Development Progress

## 项目概述 / Project Overview
- **版本**: v0.3.0
- **目标**: 为嵌入式系统设计的轻量级编程语言
- **状态**: MVP 开发中

## 已完成功能 / Completed Features

### v0.2.0
- 多行注释 (`/* */`)
- for 循环
- 数组和索引访问
- 字符串与数字连接

### v0.3.0
- 结构体定义和使用 `struct Point { x: i32, y: i32 }`
- 枚举定义和使用 `enum Color { Red, Green, Blue }`
- 结构体实例化 `Point { x: 10, y: 20 }`
- 限定枚举名 `Color::Red`
- 基本 match 表达式（数字模式）
- 添加 Token: `Struct`, `Enum`, `Match`, `ColonColon`, `Underscore`
- AST 添加: `Stmt::StructDef`, `Stmt::EnumDef`, `Expr::StructInit`, `MatchCase`, `MatchPattern`
- 解释器添加: `Value::Struct(HashMap)`, `Value::Enum(String, String)`

## 修复的问题 / Fixed Issues
1. **Struct 字段逗号** - 在 `parse_struct_def` 中添加逗号处理
2. **Enum 变体逗号** - 在 `parse_enum_def` 中添加逗号处理
3. **限定枚举名** - 添加 `::` token 解析和语义分析
4. **解释器枚举值** - 添加 `Value::Enum` 用于运行时表示枚举
5. **Match 表达式标识符模式** - 修复解析器在 `let` 语句后错误地将 `identifier { ... }` 解析为结构体初始化的问题。添加 `has_module_or_field` 标志和大写字母开头标识符检查
6. **Match 表达式 EnumName::VariantName 模式** - 修复解析器支持 `Color::Red -> 1` 形式的限定枚举名匹配。修复了解析器和解释器
7. **分号处理** - 修复解析器在语句末尾遇到分号时错误地报"expected expression"的问题。添加分号跳过逻辑

## 已完成功能 / Completed Features (续)

### v0.3.1
- 完整的 match 枚举模式支持 `Color::Red -> 1`
- 解释器支持 `Value::Enum` 与 `MatchPattern::EnumVariant` 匹配
- 字段访问表达式 `p.x` 支持
- 嵌套结构体字段访问 `p.inner.x`
- 数组索引访问 `arr[0]`
- `break` 和 `continue` 语句支持
- `loop` 循环（无限循环）支持
- `return 42` 表达式返回值支持
- 内置 `len()` 函数（支持字符串和数组）
- 内置 `empty()` 函数（检查数组/字符串是否为空）
- 内置 `is_empty()` 函数（检查数组/字符串是否为空，返回 Boolean）
- 内置 `first()` 函数（获取数组/字符串首元素）
- 内置 `last()` 函数（获取数组/字符串末元素）
- 内置 `push()` 函数（数组添加元素）
- 内置 `pop()` 函数（数组移除元素）
- 内置 `append()` 函数（拼接两个数组）
- 内置 `unshift()` 函数（在数组开头插入元素）
- 内置 `includes()` 函数（检查数组是否包含元素）
- 内置 `find_index()` 函数（查找元素索引）
- 内置 `contains()` 函数（字符串包含检查，返回 Boolean）
- 内置 `index_of()` 函数（查找子串位置）
- 内置 `last_index_of()` 函数（查找最后一个子串位置）
- 内置 `split()` 函数（字符串分割）
- 内置 `reverse()` 函数（反转数组或字符串）
- 内置 `slice()` 函数（数组/字符串切片）
- 内置 `char_at()` 函数（获取指定索引字符）
- 内置 `insert()` 函数（在指定位置插入元素）
- 内置 `remove()` 函数（移除指定位置元素）
- 内置 `take()` 函数（获取前 n 个元素）
- 内置 `drop()` 函数（丢弃前 n 个元素）
- 内置 `range()` 函数（生成数字序列）
- 内置 `lines()` 函数（按行分割字符串）
- 内置 `flatten()` 函数（展平嵌套数组）
- 内置 `unique()` 函数（去除数组重复元素）
- 内置 `any()` 函数（检查是否有真值）
- 内置 `all()` 函数（检查是否全部为真值）
- 内置 `zip()` 函数（合并两个数组）
- 内置 `enumerate()` 函数（枚举数组元素）
- 内置 `chunk()` 函数（分块数组）
- 内置 `average()` 函数（数组平均值）
- 内置 `find()` 函数（查找数组元素）
- 内置 `filter()` 函数（过滤数组元素）
- 内置 `reduce()` 函数（数组聚合运算）
- 内置 `chars()` 函数（字符串转字符数组）
- 内置 `codes()` 函数（字符串转Unicode码点数组）
- 内置 `chr()` 函数（Unicode码点转字符）
- 内置 `to_chars()` 函数（字符串转字符数组）
- 内置 `to_codes()` 函数（字符串转码点数组）
- 内置 `capitalize()` 函数（首字母大写）
- 内置 `is_alpha()` 函数（检查是否全为字母）
- 内置 `is_digit()` 函数（检查是否全为数字）
- 内置 `is_space()` 函数（检查是否全为空白）
- 内置 `abs_diff()` 函数（绝对值差）
- 内置 `mod()` 函数（取模）
- 内置 `gcd()` 函数（最大公约数）
- 内置 `lcm()` 函数（最小公倍数）
- 内置 `is_negative()` 函数（检查是否为负数）
- 内置 `is_positive()` 函数（检查是否为正数）
- 内置 `is_zero()` 函数（检查是否为零）
- 内置 `increment()` 函数（加1）
- 内置 `decrement()` 函数（减1）
- 内置 `repeat()` 函数（重复字符串）
- 内置 `replace_all()` 函数（替换所有匹配）
- 内置 `trim_start()` 函数（去除开头空白）
- 内置 `trim_end()` 函数（去除末尾空白）
- 内置 `pad_start()` 函数（字符串开头填充）
- 内置 `pad_end()` 函数（字符串末尾填充）
- 内置 `sum()` 函数（数组求和）
- 内置 `product()` 函数（数组求积）
- 内置 `sort()` 函数（数组排序）
- 内置 `to_uppercase()` 函数（转大写）
- 内置 `to_lowercase()` 函数（转小写）
- 内置 `starts_with()` 函数（是否以某字符串开头）
- 内置 `ends_with()` 函数（是否以某字符串结尾）
- 内置 `to_string()` 函数（任意值转字符串）
- 新增 `Value::Boolean` 类型（true/false）
- 内置 `abs()` 函数（绝对值）
- 内置 `min()` 函数（最小值）
- 内置 `max()` 函数（最大值）
- 内置 `pow()` 函数（幂运算）
- 内置 `floor()` 函数（向下取整）
- 内置 `ceil()` 函数（向上取整）
- 内置 `round()` 函数（四舍五入）
- 内置 `sqrt()` 函数（平方根）
- 内置 `to_i32()` 函数（转换为整数）
- 内置 `to_f64()` 函数（转换为浮点数）
- 内置 `parse_int()` 函数（解析字符串为整数）
- 内置 `parse_float()` 函数（解析字符串为浮点数）
- 内置 `is_number()` 函数（检查是否为数字）
- 内置 `is_string()` 函数（检查是否为字符串）
- 内置 `is_array()` 函数（检查是否为数组）
- 内置 `is_boolean()` 函数（检查是否为布尔值）
- 内置 `typeof()` 函数（获取值类型名称）
- 内置 `clamp()` 函数（限制值在范围内）
- 负数字面量支持（如 `-5`、`-3.14`）
- 改进的值显示格式（数字显示为 `42` 而非 `Number(42.0)`）

## 测试状态 / Test Status
- **Lexer 测试**: 14 个全部通过
- **集成测试**: demo.light, struct_test.light, enum_qualified.light, match_enum.light 等正常工作

## 下一步 / Next Steps
1. 优化解释器性能
2. 添加更多内置函数
3. 更新文档和示例

## 相关文件 / Relevant Files
- `examples/struct_test.light` - 结构体测试
- `examples/enum_qualified.light` - 枚举测试
- `examples/color.light` - 简单枚举测试
- `examples/demo.light` - 演示程序

## 构建和测试命令
```bash
cargo build      # 构建
cargo test       # 运行测试
cargo run -- run examples/demo.light  # 运行示例
```
