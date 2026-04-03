# ModelGate 设计令牌

**版本:** 1.1  
**日期:** 2026年4月

> **实现说明：** 控制台使用 TailwindCSS / shadcn 主题变量，色值可能与下表不完全一致；以 `frontend` 中 CSS 变量与组件为准，下表作品牌与语义参考。

---

## 一、颜色系统

### 1.1 品牌色

| 名称 | 色值 | 用途 |
|------|------|------|
| Primary | `#1890FF` | 主按钮、链接、选中态 |
| Primary-Hover | `#40A9FF` | 主按钮悬停 |
| Primary-Active | `#096DD9` | 主按钮按下 |
| Primary-Bg | `#E6F7FF` | 主色背景、选中背景 |

### 1.2 功能色

| 名称 | 色值 | 用途 |
|------|------|------|
| Success | `#52C41A` | 成功状态 |
| Success-Bg | `#F6FFED` | 成功背景 |
| Warning | `#FAAD14` | 警告状态 |
| Warning-Bg | `#FFFBE6` | 警告背景 |
| Error | `#FF4D4F` | 错误状态 |
| Error-Bg | `#FFF2F0` | 错误背景 |
| Info | `#1890FF` | 信息提示 |
| Info-Bg | `#E6F7FF` | 信息背景 |

### 1.3 中性色

| 名称 | 色值 | 用途 |
|------|------|------|
| Title | `#262626` | 标题文字 |
| Primary-Text | `#262626` | 主要文字 |
| Secondary-Text | `#595959` | 次要文字 |
| Disabled-Text | `#BFBFBF` | 禁用文字 |
| Border | `#D9D9D9` | 边框 |
| Divider | `#F0F0F0` | 分割线 |
| Background | `#F5F5F5` | 页面背景 |
| Component-Bg | `#FFFFFF` | 组件背景 |
| Mask | `rgba(0,0,0,0.45)` | 遮罩层 |

### 1.4 渠道状态色

| 状态 | 色值 | 说明 |
|------|------|------|
| 正常 | `#52C41A` | 服务正常 |
| 慢响应 | `#FAAD14` | 响应时间 > 5s |
| 故障 | `#FF4D4F` | 服务不可用 |
| 禁用 | `#BFBFBF` | 已禁用 |

---

## 二、字体系统

### 2.1 字体家族

```css
font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto,
             'Helvetica Neue', Arial, 'Noto Sans', sans-serif,
             'Apple Color Emoji', 'Segoe UI Emoji';
```

### 2.2 代码字体

```css
font-family: 'SF Mono', 'Monaco', 'Inconsolata', 'Fira Mono',
             'Droid Sans Mono', 'Source Code Pro', monospace;
```

### 2.3 字号规范

| 名称 | 字号 | 行高 | 字重 | 用途 |
|------|------|------|------|------|
| H1 | 24px | 32px | 600 | 页面标题 |
| H2 | 20px | 28px | 600 | 区块标题 |
| H3 | 16px | 24px | 600 | 小标题 |
| Body | 14px | 22px | 400 | 正文 |
| Body-Small | 12px | 20px | 400 | 辅助文字 |
| Caption | 12px | 20px | 400 | 说明文字 |
| Overline | 10px | 16px | 400 | 标签文字 |

---

## 三、间距系统

### 3.1 基础单位

**基础单位：4px**

| 名称 | 值 | 用途 |
|------|-----|------|
| xs | 4px | 紧凑间距 |
| sm | 8px | 元素内间距 |
| md | 12px | 小组件间距 |
| lg | 16px | 标准间距 |
| xl | 24px | 大间距 |
| xxl | 32px | 区块间距 |
| xxxl | 48px | 页面边距 |

### 3.2 页面布局间距

| 区域 | 间距 |
|------|------|
| 页面边距 | 24px |
| 卡片内边距 | 16px |
| 卡片间距 | 16px |
| 表单行间距 | 24px |
| 表格单元格内边距 | 16px 8px |

---

## 四、圆角规范

| 名称 | 值 | 用途 |
|------|-----|------|
| sm | 2px | 小按钮、标签 |
| md | 4px | 按钮、输入框、卡片 |
| lg | 8px | 弹窗、下拉面板 |
| xl | 12px | 大卡片 |
| full | 9999px | 圆形、胶囊形 |

---

## 五、阴影规范

| 名称 | 值 | 用途 |
|------|-----|------|
| sm | `0 1px 2px rgba(0,0,0,0.03)` | 卡片默认 |
| md | `0 4px 12px rgba(0,0,0,0.08)` | 卡片悬停 |
| lg | `0 6px 16px rgba(0,0,0,0.08)` | 下拉面板 |
| xl | `0 12px 24px rgba(0,0,0,0.12)` | 弹窗 |

---

## 六、图标规范

### 6.1 尺寸

| 名称 | 尺寸 | 用途 |
|------|------|------|
| sm | 12px | 行内图标 |
| md | 16px | 伴随文字、按钮图标 |
| lg | 20px | 导航图标 |
| xl | 24px | 功能图标 |
| xxl | 32px | 空状态图标 |

### 6.2 图标风格

- 线性图标，线宽 1.5px
- 圆角连接
- 视觉大小 16×16（含 2px 留白）

---

## 七、按钮规范

### 7.1 主按钮

| 状态 | 背景 | 边框 | 文字 | 阴影 |
|------|------|------|------|------|
| Default | `#1890FF` | - | `#FFFFFF` | - |
| Hover | `#40A9FF` | - | `#FFFFFF` | - |
| Active | `#096DD9` | - | `#FFFFFF` | - |
| Disabled | `#F5F5F5` | - | `#BFBFBF` | - |
| Loading | `#1890FF` | - | `#FFFFFF` | - |

### 7.2 次按钮

| 状态 | 背景 | 边框 | 文字 |
|------|------|------|------|
| Default | `#FFFFFF` | `#D9D9D9` | `#595959` |
| Hover | `#FFFFFF` | `#1890FF` | `#1890FF` |
| Active | `#FFFFFF` | `#096DD9` | `#096DD9` |
| Disabled | `#F5F5F5` | `#D9D9D9` | `#BFBFBF` |

### 7.3 危险按钮

| 状态 | 背景 | 文字 |
|------|------|------|
| Default | `#FF4D4F` | `#FFFFFF` |
| Hover | `#FF7875` | `#FFFFFF` |
| Active | `#CF1322` | `#FFFFFF` |
| Disabled | `#F5F5F5` | `#BFBFBF` |

### 7.4 按钮尺寸

| 尺寸 | 高度 | 内边距 | 字号 |
|------|------|--------|------|
| Small | 24px | 7px 8px | 12px |
| Default | 32px | 8px 16px | 14px |
| Large | 40px | 12px 24px | 16px |

---

## 八、表单组件规范

### 8.1 输入框

| 状态 | 边框 | 背景 | 阴影 |
|------|------|------|------|
| Default | `#D9D9D9` | `#FFFFFF` | - |
| Hover | `#40A9FF` | `#FFFFFF` | - |
| Focus | `#40A9FF` | `#FFFFFF` | `0 0 0 2px rgba(24,144,255,0.2)` |
| Error | `#FF4D4F` | `#FFFFFF` | `0 0 0 2px rgba(255,77,79,0.2)` |
| Success | `#52C41A` | `#FFFFFF` | - |
| Disabled | `#D9D9D9` | `#F5F5F5` | - |

**输入框尺寸：**

| 尺寸 | 高度 | 内边距 | 字号 |
|------|------|--------|------|
| Small | 24px | 4px 8px | 12px |
| Default | 32px | 6px 12px | 14px |
| Large | 40px | 8px 12px | 16px |

**错误提示：**
- 位置：输入框下方
- 颜色：`#FF4D4F`
- 字号：12px
- 图标：感叹号

### 8.2 选择器

**下拉选择：**

| 状态 | 边框 |
|------|------|
| Default | `#D9D9D9` |
| Hover | `#40A9FF` |
| Focus | `#40A9FF` + 阴影 |
| Disabled | `#D9D9D9` |

**下拉面板：**
- 背景：`#FFFFFF`
- 阴影：`0 6px 16px rgba(0,0,0,0.08)`
- 圆角：8px
- 选项高度：32px
- 选项悬停：`#F5F5F5`
- 选项选中：`#E6F7FF`

### 8.3 复选框

| 状态 | 边框 | 背景 | 勾选 |
|------|------|------|------|
| Unchecked | `#D9D9D9` | `#FFFFFF` | - |
| Checked | `#1890FF` | `#1890FF` | `#FFFFFF` |
| Indeterminate | `#1890FF` | `#1890FF` | 横线 |
| Disabled | `#D9D9D9` | `#F5F5F5` | `#BFBFBF` |

### 8.4 开关

| 状态 | 背景 | 圆点位置 |
|------|------|---------|
| Off | `#BFBFBF` | 左侧 |
| On | `#1890FF` | 右侧 |
| Disabled | `#F5F5F5` | - |
| Loading | `#1890FF` + 旋转 | - |

**尺寸：** 44px × 22px，圆点 18px

---

## 九、标签规范

| 类型 | 背景 | 边框 | 文字 |
|------|------|------|------|
| Default | `#FAFAFA` | `#D9D9D9` | `#595959` |
| Success | `#F6FFED` | `#B7EB8F` | `#52C41A` |
| Warning | `#FFFBE6` | `#FFE58F` | `#FAAD14` |
| Error | `#FFF2F0` | `#FFCCC7` | `#FF4D4F` |
| Info | `#E6F7FF` | `#91D5FF` | `#1890FF` |

**可关闭标签：**
- 关闭图标在右侧
- Hover 显示关闭图标
- 关闭图标悬停变红

---

## 十、表格规范

### 10.1 表头

- 背景：`#FAFAFA`
- 文字：`#262626`
- 字重：500
- 高度：48px
- 边框底部：`#F0F0F0`

### 10.2 表格行

| 状态 | 背景 |
|------|------|
| Default | `#FFFFFF` |
| Hover | `#FAFAFA` |
| Selected | `#E6F7FF` |
| Striped | `#FAFAFA`（奇数行）|

### 10.3 表格单元格

- 高度：48px
- 内边距：16px 8px
- 边框底部：`#F0F0F0`

---

## 十一、弹窗规范

### 11.1 结构

- 宽度：400px / 600px / 800px
- 圆角：8px
- 阴影：`0 12px 24px rgba(0,0,0,0.12)`
- 背景：`#FFFFFF`

### 11.2 标题栏

- 高度：56px
- 内边距：0 24px
- 字号：16px
- 字重：500
- 底部边框：`#F0F0F0`
- 关闭按钮：右侧，24×24px

### 11.3 内容区

- 内边距：24px
- 最大高度：70vh
- 超出滚动

### 11.4 底部栏

- 高度：64px
- 内边距：0 24px
- 按钮右对齐
- 按钮间距：8px

---

## 十二、Toast 提示规范

### 12.1 结构

- 高度：auto（最小 40px）
- 最大宽度：400px
- 圆角：4px
- 阴影：`0 4px 12px rgba(0,0,0,0.15)`
- 内边距：12px 16px

### 12.2 类型样式

| 类型 | 背景 | 图标 |
|------|------|------|
| Success | `#F6FFED` | ✅ `#52C41A` |
| Error | `#FFF2F0` | ❌ `#FF4D4F` |
| Warning | `#FFFBE6` | ⚠️ `#FAAD14` |
| Info | `#E6F7FF` | ℹ️ `#1890FF` |

### 12.3 动画

- 入场：从顶部滑入，250ms ease-out
- 出场：淡出上移，250ms ease-in
- 自动消失：3秒

---

**文档结束**
