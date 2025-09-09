# Meter Core 配置系统

## 📁 配置文件位置

### 独立运行模式
配置文件按以下优先级搜索：
1. 当前工作目录：`./config.json`
2. 可执行文件所在目录：`{exe_dir}/config.json`

### Tauri 应用模式
配置文件按以下优先级搜索：
1. 可执行文件所在目录：`{exe_dir}/config.json`
2. 项目配置目录：`../meter-core/config.json`

### 开发模式
当运行 `pnpm tauri dev` 时，配置文件会自动复制到：
```
src-tauri/target/debug/config.json
```

### 生产模式
当运行生产构建时，配置文件会通过Tauri的资源系统自动包含在应用包中。

## 🔧 配置选项

### 日志配置
```json
{
  "logging": {
    "level": "debug",           // 日志级别: trace, debug, info, warn, error
    "enable_file_logging": true, // 是否启用文件日志
    "log_file_path": "logs/meter-core.log", // 日志文件路径
    "max_log_files": 5,         // 最大日志文件数量
    "max_log_size": 10,         // 单个日志文件最大大小(MB)
    "enable_console_logging": true // 是否启用控制台日志
  }
}
```

### 数据包捕获配置
```json
{
  "packet_capture": {
    "filter": "ip and tcp",     // WinDivert过滤器
    "buffer_size": 10485760,   // 缓冲区大小(10MB)
    "mtu": 65535,              // MTU大小(最大以太网帧)
    "enable_tcp_reassembly": true, // 是否启用TCP重组
    "max_connections": 10000,  // 最大连接数
    "connection_timeout": 300  // 连接超时时间(秒)
  }
}
```

### Web服务器配置
```json
{
  "web_server": {
    "host": "127.0.0.1",       // 服务器主机
    "port": 8989,              // 服务器端口
    "enable_cors": true,       // 是否启用CORS
    "enable_websocket": true,  // 是否启用WebSocket
    "static_files_path": "public", // 静态文件路径
    "request_timeout": 30      // 请求超时时间(秒)
  }
}
```

### 数据管理配置
```json
{
  "data_manager": {
    "cache_file_path": "users.json",     // 用户缓存文件路径
    "settings_file_path": "settings.json", // 设置文件路径
    "skill_config_path": "tables/skill_names.json", // 技能配置路径
    "auto_save_interval": 300,           // 自动保存间隔(秒)
    "max_cache_age": 30,                 // 缓存最大年龄(天)
    "enable_persistence": true           // 是否启用持久化
  }
}
```

## 📋 使用方法

### 1. 复制示例配置文件
```bash
cp meter-core/config.example.json meter-core/config.json
```

### 2. 编辑配置
使用你喜欢的编辑器修改 `meter-core/config.json` 中的设置。

### 3. 开发模式测试
```bash
pnpm tauri dev
```
配置文件会自动复制到开发目录。

### 4. 生产构建
```bash
pnpm tauri build
```
配置文件会自动包含在最终的应用包中。

## 🎯 配置优先级

配置系统采用简化的多级覆盖，按以下优先级从高到低：

1. **配置文件**：`config.json` 中的设置（最高优先级）
2. **环境变量**：`METER_CORE_*` 环境变量
3. **默认值**：代码中定义的默认值

### 环境变量列表
- `METER_CORE_HOST`：Web服务器主机
- `METER_CORE_PORT`：Web服务器端口
- `METER_CORE_LOG_LEVEL`：日志级别
- `METER_CORE_INTERFACE`：网络接口过滤器

## 📝 调试配置

要启用详细日志输出，将日志级别设置为 `debug`：

```json
{
  "logging": {
    "level": "debug"
  }
}
```

这将显示：
- 数据包过滤详情
- TCP流重组信息
- 服务器识别过程
- 性能统计信息

## 🚨 注意事项

1. **配置文件格式**：必须是有效的JSON格式
2. **路径问题**：在生产模式下，所有路径都是相对于应用可执行文件的
3. **权限问题**：某些配置（如网络过滤器）可能需要管理员权限
4. **重启要求**：配置文件修改后需要重启应用才能生效

## 🐛 故障排除

### 配置文件不生效
1. 检查JSON格式是否正确
2. 确认文件路径是否正确
3. 查看应用日志中的错误信息

### 日志级别不生效
1. 确认配置文件中的 `logging.level` 设置
2. 检查是否有命令行参数覆盖了配置文件
3. 验证环境变量是否设置

### 数据包捕获失败
1. 确认WinDivert驱动已正确安装
2. 检查应用程序是否以管理员权限运行
3. 验证 `packet_capture.filter` 配置是否正确
