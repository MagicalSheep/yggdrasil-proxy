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
  ls: https://littleskin.cn/api/yggdrasil
main: ls
enableMasterSlaveMode: true
```

- `meta`: 遵循 [Yggdrasil API](https://github.com/yushijinhun/authlib-injector/wiki/Yggdrasil-%E6%9C%8D%E5%8A%A1%E7%AB%AF%E6%8A%80%E6%9C%AF%E8%A7%84%E8%8C%83) 的元数据定义。
- `dataSource`: 数据库地址，由协议、用户名、密码、地址、数据库名组成，目前仅支持`MySql`协议。请在运行前确保数据库已正确创建。
- `secret`: 用于对代理分发的 `accessToken` 进行签名，代理分发的 `accessToken` 属于 `JWT`。
- `address`: 代理端监听的 `IPv4` 地址。
- `port`: 代理端监听的端口。
- `backends`: 源后端服务器，由多个遵循 [Yggdrasil API](https://github.com/yushijinhun/authlib-injector/wiki) 接口的服务器地址组成。其中 `key` 值将被用于区分源端及重命名玩家，当前暂不支持自定义重命名策略，所有通过代理端的玩家将被重命名为 `{Backend Server Key}_{Player Name}`。
- `main`: 启用主从模式时的主源服务器。
- `enableMasterSlaveMode`: 是否启用主从模式。

## 主从模式

启用主从模式后，与被选定为主源的 Yggdrasil 服务器之间的通信将不再经过代理程序的修改（重签名除外，这是正常显示皮肤的必要修改），即角色的 UUID 及名称将与源服务器保持一致。名称不再带有前缀，UUID 也不会被其他服务器角色先行占用。

由于可以通过在源服务器中构造与代理为其他服务器角色修改后的相同角色名称，即：来自主源服务器 A 的玩家主动使用名称 `B_sheep`，而来自 B 服务器的玩家名称为 `sheep`，若不作处理，则他们将在游戏里将拥有相同的名称 `B_sheep`，这是不可接受的。

因此，当这种情况发生时，代理将阻止 B 服务器的玩家 `sheep` 加入服务器，仅保障主源服务器玩家正常游戏。

你可以随时通过修改配置文件来切换主源服务器，这将不会带来任何副作用。

不过需要注意，由于绝大多数采用 `BlessSkin Yggdrasil API` 插件的服务器使用 `Version 3 UUID` 算法生成角色 UUID，这意味着同一名称的玩家在这些不同的服务器上会拥有相同的 UUID。如果你切换了主源服务器至 B，来自 B 服务器上的 sheep 将拥有原主源服务器 A 的 sheep 的游戏数据，请确保这两个 sheep 的角色拥有者是同一位玩家。

关闭主从模式时，所有角色的 UUID 将重新随机生成，每个角色的名称都将携带其源服务器名的前缀。你可以随时启用和关闭主从模式，这不会带来任何副作用。

## 代理如何工作

位于中间的代理在本地存储和处理代理端与各源端用户数据的映射关系，利用协议中具有相当自由度的 `accessToken` 实现分流与状态记录。

具体而言，代理向客户端重新分发了 `accessToken`，其中记录了客户端在所有后端服务器的 `accessToken` 凭证及登录状态，由签名确定其所有权。利用这些状态，代理可以确定分流的后端服务器，并当 `Profile` 在中间传递时，进行重签名及处理冲突UUID等操作。

基于该原理，游戏服务器所使用的代理应具有安全的来源。