# Yggdrasil Server Proxy

为遵循 [Yggdrasil API](https://github.com/yushijinhun/authlib-injector) 接口的服务器实现反向代理。

## 安装及使用

从 [Release](https://github.com/MagicalSheep/yggdrasil-proxy/releases) 页下载对应平台的二进制文件，直接运行。

首次运行会在运行目录生成配置文件 `config.yaml` 与私钥文件 `private_key.pem`，填写配置文件中的数据库地址、源服务器地址等必要信息后，重新运行程序即开始工作。

## 配置文件

```yaml
meta:
  serverName: Union Authenticate Server
  links:
    homepage: https://example.com
    register: https://example.com/auth/register
  feature.non_email_login: true
  feature.legacy_skin_api: false
  feature.no_mojang_namespace: false
  feature.enable_mojang_anti_features: false
  feature.enable_profile_key: false
  feature.username_check: false
  skinDomains:
  - littleskin.cn
  - skin.prinzeugen.net
  - example.com
dataSource: mysql://root:password@localhost/database
secret: example-token-secret
address: 0.0.0.0
port: 8080
backends:
  example: https://example.com/api/yggdrasil
  l-skin: https://littleskin.cn/api/yggdrasil
```

- `meta`: 遵循 [Yggdrasil API](https://github.com/yushijinhun/authlib-injector/wiki/Yggdrasil-%E6%9C%8D%E5%8A%A1%E7%AB%AF%E6%8A%80%E6%9C%AF%E8%A7%84%E8%8C%83) 的元数据定义。
- `dataSource`: 数据库地址，由协议、用户名、密码、地址、数据库名组成，目前仅支持`MySql`协议。请在运行前确保数据库已正确创建。
- `secret`: 用于对代理分发的 `accessToken` 进行签名，代理分发的 `accessToken` 属于 `JWT`。
- `address`: 代理端监听的 `IPv4` 地址。
- `port`: 代理端监听的端口。
- `backends`: 源后端服务器，由多个遵循 [Yggdrasil API](https://github.com/yushijinhun/authlib-injector/wiki) 接口的服务器地址组成。其中 `key` 值将被用于区分源端及重命名玩家，当前暂不支持自定义重命名策略，所有通过代理端的玩家将被重命名为 `{Backend Server Key}_{Player Name}`。

## 代理如何工作

位于中间的代理在本地存储和处理代理端与各源端用户数据的映射关系，利用协议中具有相当自由度的 `accessToken` 实现分流与状态记录。

具体而言，代理向客户端重新分发了 `accessToken`，其中记录了客户端在所有后端服务器的 `accessToken` 凭证及登录状态，由签名确定其所有权。利用这些状态，代理可以确定分流的后端服务器，并当 `Profile` 在中间传递时，进行重签名及处理冲突UUID等操作。

基于该原理，游戏服务器所使用的代理应具有安全的来源。