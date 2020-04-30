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
---

# 记录下每次配环境的操作，减少环境配置时间

- [zsh](#zsh)
- [ssh](#ssh)
- [golang](#golang)
- [neovim/spacevim](#neovim)
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
    tar -C /usr/local -zxvf go1.4.2.linux-amd64.tar.gz
    ```
- 配置go环境变量, 将一下两行追加至`/etc/profile`中
        
    ```bash
    export GOROOT=/usr/local/go
    export PATH=$PATH:$GOROOT/bin
    ```
- 配置go环境

    ```bash
    go env -w GO111MODULE=on
    go env -w GOPROXY="https://goproxy.cn,direct"
    ```

## neovim
> 本来一直在用vim，neovim更加友好就决定迁移了
> - 一些改变:
> 1. 默认encoding: neovim `utf-8`, vim `latin1`
> 2. 配置目录: neovim $XDG_CONFIG_HOME/nvim/init.vim 和$XDG_CONFIG_HOME/nvim, vim 为$HOME/.vimrc
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


​    