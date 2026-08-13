---
title: "My Linux"
date: 2020-04-12T11:25:07+08:00
draft: false
toc: false
images:
tags:
  - linux
  - golang
  - neovim
  - vim
  - zsh
  - oh-my-zsh
  - ssh
  - spacevim
  - mysql
---

# 记录下每次配环境的操作，减少环境配置时间

- [zsh](#zsh)
- [ssh](#ssh)
- [golang](#golang)
- [neovim/spacevim](#neovim)
- [mysql](#mysql)
***
## zsh
- 安装 git 与 zsh
  
    ```bash
    sudo apt install git zsh
    ```
- 安装 oh-my-zsh
1. curl

    ```bash
    sh -c "$(curl -fsSL https://raw.github.com/ohmyzsh/ohmyzsh/master/tools/install.sh)"
    ```
2. wget

    ```bash
    sh -c "$(wget https://raw.github.com/ohmyzsh/ohmyzsh/master/tools/install.sh -O -)"
    ```
- 安装插件
1. zsh-autosuggestions

    ```bash
    git clone https://github.com/zsh-users/zsh-autosuggestions $ZSH_CUSTOM/plugins/zsh-autosuggestions
    ```
2. zsh-syntax-highlighting

    ```bash
    git clone https://github.com/zsh-users/zsh-syntax-highlighting.git ${ZSH_CUSTOM:-~/.oh-my-zsh/custom}/plugins/zsh-syntax-highlighting
    ```
## ssh
1. 配置 ssd_config

        PermitRootLogin yes // 别跟我说 Root不安全，炸了重装就完事了😈
        PubkeyAuthentication yes 
2. 上传公钥文件 将公钥文件追加到 authorized_keys中
3. 注意权限 (埋个坑: 为什么权限要这样设置?)
    1. .ssh 权限应为 700 
    2. authorized_keys 应为 600
***
## golang
- 下载安装包
1. curl
   
    ```bash
    curl -O https://dl.google.com/go/go1.14.2.linux-amd64.tar.gz
    ```
2. wget 
   
    ```bash
    wget https://dl.google.com/go/go1.14.2.linux-amd64.tar.gz
    ```
- 解压

    ```bash
    tar -C /usr/local -zxvf go1.14.2.linux-amd64.tar.gz
    ```
- 配置go环境变量, 将一下两行追加至`/etc/profile`中
        
    ```bash
    export GOROOT=/usr/local/go
    export PATH=$PATH:$GOROOT/bin
    export GOPATH=$(go env GOPATH)
    export PATH=$PATH:$GOPATH/bin
    ```
- 配置go环境

    ```bash
    go env -w GO111MODULE=on
    go env -w GOPROXY="https://goproxy.cn,direct"
    ```

## neovim
> 本来一直在用 vim，neovim 更加友好就决定迁移了
> - 一些改变:
> 1. 默认 encoding: neovim `utf-8`, vim `latin1`
> 2. 配置目录: neovim $XDG_CONFIG_HOME/nvim/init.vim 和$XDG_CONFIG_HOME/nvim, vim 为$HOME/.vimrc
> 3. `:version` 查看 neovim 版本信息, `:checkhealth` 查看 neovim 健康状态, `:help init.vim` 查看 neovim 配置文件相关信息.
> 4. Windows 下 `scooop install neovim`, `:help init.vim` 发现目录在  `~/AppData/Local/nvim/init.vim`, 而官网的 install.cmd 会将 [SpaceVim](https://github.com/SpaceVim/SpaceVim) clone 到 ~./.SpaceVim 目录下(install.cmd 有创建`$HOME\.SpaceVim` -> `$HOME\AppData\Local\nvim` 的软链接的, 但似乎没创建成功), 所以 neovim 无法正确读取配置. 可以通过手动 `git clone https://github.com/SpaceVim/SpaceVim.git $HOME/AppData/Local/nvim` 来解决.
- 安装

    ```bash
    sudo apt install neovim
    ```

- 用SpaceVim配置 [SpaceVim中文文档](https://spacevim.org/cn/)
> 什么? 为什么不自己配？ 
> : 我懒😎

1. 安装     
  
    ```bash
    curl -sLf https://spacevim.org/cn/install.sh | bash 
    ```
2. 把 vi 或 vim 转为使用 neovim 
> 你以为我在用 vim， 实际上我用的是 neovim 😎

        vim ~/.zshrc
        alias vi='nvim'
        alias vim='nvim'
        source ~/.zshrc

## Mysql

- 安装

    ```bash
    sudo apt install mysql-server
    ```

- 配置文件修改

    ~~~bash
    vim /etc/mysql/mysql.conf.d/mysqld.cnf
  
    # 修改如下
  
    # bind-address = 127.0.0.1
    ~~~

  

- 用户配置

    ~~~mysql
    use mysql;
    # 修改密码
    alter user `root`@`localhost` identified with mysql_native_password by 'zjygogogo';
  
    # 设置远程访问
    update user set host=`%` where user=`root`;
  
    #刷新权限
    flush privileges;
    ~~~

