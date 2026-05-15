# Web

前端基于 Next.js。

## 本地开发

```bash
corepack pnpm dev
```

默认访问地址：

- `http://127.0.0.1:3000`

如果需要给局域网内其他设备访问，使用宿主机 IP：

- `http://192.168.21.97:3000`

## 相关环境变量

- `NEXT_PUBLIC_API_BASE_URL`: 浏览器访问 API 的外部地址，当前应使用宿主机 IP，例如 `http://192.168.21.97:8080`
- `API_INTERNAL_BASE_URL`: Web 容器内部访问 API 的地址，Compose 下固定为 `http://api:8080`

## 登录跳转

- 登录和退出后的跳转地址优先使用请求头里的 `x-forwarded-host` / `host` / `x-forwarded-proto`
- 因此当你通过宿主机 IP 访问 Web 时，跳转会保持在当前宿主机 IP，不会回落到 `localhost`
- Session Cookie 的 `Secure` 属性会跟随当前请求协议；`http://<HOST_IP>:3000` 下不会错误地写成 `Secure Cookie`
