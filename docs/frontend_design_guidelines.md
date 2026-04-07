# AI Singularity 前端设计与交互规范 (Frontend Design Guidelines)

> **核心基调**：Premium Light (高定纯净浅色)
> **设计对标**：Vercel、Linear、Apple macOS 原生界面
> **最终目标**：打造极具物理通透感、空气感阴影与丝滑过渡的高端极简主义开发者工具。

---

## 1. 颜色与材质 (Color & Material)

本系统为主打明亮纯净的现代化工具。**严禁**使用大面积死板的纯黑或高饱和度的工业原色（如 `#FF0000`, `#0000FF`），所有色彩需具备“呼吸感”。

### 1.1 基础材质
- **全局背景色 (Background)**: 推荐使用绝对纯白 `#FFFFFF` 到极寒灰 `#F8FAFC` 作为底漆。
- **卡片/弹层质感 (Glassmorphism)**:
  - 核心悬浮面板（如弹窗、全局搜索框、下拉菜单）**必须**引入物理毛玻璃材质。
  - standard CSS: `background: rgba(255, 255, 255, 0.4); backdrop-filter: blur(16px);`
  - 为了提供卡片质感，必须搭配 `border: 1px solid rgba(255, 255, 255, 0.6);` 或者极为轻薄的灰色线 `rgba(15, 23, 42, 0.08)`。

### 1.2 点缀与发光色 (Accent & Glow)
- **品牌高光 (Accent)**: 使用科技纯蓝 (`#2563EB`) 或灵动紫 (`#8B5CF6`)。
- **发光投影 (Glow / Inner Glow)**: 主按钮除了阴影外，建议带有极其轻微的白色内衬高光 `box-shadow: inset 0 1px 0 rgba(255,255,255,0.2)` 提升立体感。

### 1.3 阴影刻画 (Shadows)
不要使用常规浏览器干瘪的 `0 4px 8px rgba(0,0,0,.1)`，请采用叠加扩散阴影呈现不可思议的浮空感：
- 卡片阴影: `box-shadow: 0 20px 40px -10px rgba(15, 23, 42, 0.08), 0 0 0 1px rgba(15, 23, 42, 0.03);`
- 悬停浮起阴影: `box-shadow: 0 30px 60px -12px rgba(15, 23, 42, 0.12);`

---

## 2. 交互与微动画 (Micro-Interactions)

### 2.1 缓动函数 (Easing)
放弃自带的 `ease-in-out`。所有的非线性物理弹出与位移必须使用以下缓动控制：
- **弹射出场 & 丝滑刹车 (Snappy)**:
  `transition-timing-function: cubic-bezier(0.16, 1, 0.3, 1);` 或 `cubic-bezier(0.2, 0.8, 0.2, 1);`
- 动画持续时间应当被控制在 `0.2s` - `0.35s` 之间，绝对不可拖拉。

### 2.2 Hover & Focus 效果
- **按钮**：点击时加入缩小下按感 (`transform: scale(0.97)` 或 `translateY(1px)`)，提升实体打击反馈。
- **输入框焦点 (Input Focus)**：决不允许出现系统原生的黑色或生硬细线发光环！Focus 时应当使用类似平滑光点散开的光辉（Soft Ring）：
  `box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.15); border-color: var(--color-accent);`
- **悬停状态 (Hover)**：大区块的背景悬停请极其克制，如 `rgba(15, 23, 42, 0.03)`，不打扰阅读。

### 2.3 状态表达 (Loading & Success)
- **摒弃转圈圈**：在做核心提交（如 OAuth 绑定验证、配置导入）时，尽量不要只丢一个转圈圈 SVG。引入带呼吸效果的条带渐变、进度填充，或者带有脉冲放大缩小光柱的微动画序列。
- 成功时的绿色应当倾向于明亮的 `#10B981` 搭配 `opacity: 0.1` 背景的高级表达方式。

---

## 3. 组件约束 (Component Anti-Patterns)

**系统严禁使用任何影响统一体验的原生丑陋表单件。**

### 3.1 废弃 `<select>` 下拉框
完全由于不可操控外观，任何在表单或页面主体出现的“下拉选择、筛选项”，都应当使用 React / CSS 定制出的 Custom Dropdown：
1. 包含精练的圆角和细边框。
2. 展开菜单必须是高透度 `backdrop-filter: blur(12px)`。
3. 悬停菜单子项时增加微交互高亮，不可破坏边距。

### 3.2 自定义滚动条 (Scrollbar)
容器超出视界时，**绝对不可使用浏览器默认巨大的丑陋滚动条**。请全局统一为隐藏滚动条轨道、当且仅当发生滚动/悬空才现出浅细灰白把手的滚动条 (`::-webkit-scrollbar`) 方案。

---

> 此文档即日起作为该项目前沿技术栈下全部 React & CSS / Tailwind 开发的基础红线法则。所有新加入的功能页面或组件应以对标本文档要求为默认基准。
