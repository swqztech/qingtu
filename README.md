# qingtu（轻图）

本地图片 **压缩 / 缩放 / 格式转换** 与 **横向拼接**，Rust + egui 实现。

## 构建

```bash
cargo build --release
target\release\qingtu.exe
```

## 功能

- **压缩**：质量、最长边缩放、输出 JPEG/PNG/WebP 或保持原格式
- **拼接**：多张图按顺序横向拼接（高度对齐）
- 默认输出到源目录下的 `compressed/`
