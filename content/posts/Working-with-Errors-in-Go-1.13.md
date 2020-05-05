---
title: "Working With Errors in Go 1.13"
date: 2020-05-04T17:33:51+08:00
draft: true
toc: false
images:
tags:
  - untagged
---

本篇为[Working With Errors in Go 1.13]("https://blog.golang.org/go1.13-errors")的翻译

为何翻译?

> 谁愿意天天对着英文看呢？翻译后天天看中文，舒服！😄
>
> 看原文就点上上面那个哦⬆

下面开始翻译

原作者: *Damien Neil and Jonathan Amsterdam*

发表时间: *17 October 2019*

### Introduction

过去的十年里Go的 [erors as value](https://blog.golang.org/errors-are-values) 为我们服务的很好. 尽管标准库对 errors 的支持一直很小—只有 `errors.New` 和`fmt.Errorf` 这两个函数, 他们只能生成带一段 `message` 的 `error` — `built-in` 的 `error` 接口允许 Go 开发者 添加任何他们想要的信息. 这样做只需要一个实现了 `Error` 的类型.

```go
type QueryError struct {
    Query string
    Err	  error
}

func (e *QueryError) Error() string {return e.Query + ":" + e.Err.Error()}
```

像这样的 `Error` 类型是无所不在的, 并且他们可以储存各种各样的信息, 从 `timestamps` 到 `filenames` 再到 `address`. 通常, 通常该信息中包括另一个交低级的错误以提供额外的上下文.

在 Go 的代码中, 这种一个 `error` 包含着另一个的设计模式无数不在, 以至于在[广泛的讨论后](https://github.com/golang/go/issues/29934), Go1.13 额外加入了对它的支持. 这篇文章描述了为了提供这个支持在标准库中额外添加的内容: `error` 包中三个新的函数和`fmt.Errorf`中新的 `formatting verb`(格式化动词... 总感觉这么翻很奇怪就放英文了)

在详细描述这些改变之前, 让我们回顾一下在语言的早期版本中是如何检查和构建错误的.

#### Errors before Go 1.13

#### Examining errors

Go中错误为值类型. 开发者通过几种方式来基于那些值做决定. 最常见的就是将错误与 `nil` 比较, 来查看操作是否失败.

```go
if err != nil {
    // something went wrong
}
```

有些时候我们将 `error` 与一个预定义的值进行比较, 来看一个特殊的 `error` 是否发生了.

```go
var ErrNotFound = errors.New("not found")

if err == ErrNotFound {
    // something wasn't found
}
```

`error` 的值的类型可以是任何满足语言定义的 `error` 接口的类型. 程序可以用类型或者类型开关来将`error` 的值视为更特定的类型.

```go
type NotFoundError struct {
    Name string
}

func (e *NotFoundError) Error() string {return e.Name + ": not found" }

if e, ok := err.(*NotFoundError); ok {
    // e.Name wasn't found
}
```

### Adding information

通常情况下, 函数将 `error` 上抛至栈的顶层的同时向其中添加信息, 比如, 一段对错误发生状况的简短描述. 构造一个新的 `errror` 的简单方法就是包含前一个 `error` 的文本:

~~~go
if err != nil {
    return fmt.Errorf("decompress %v: %v", name, err)
}
~~~

通过 `fmt.Errorf` 创建一个新的 `error` 会对其原 `error` 中文本之外的所有信息. 正如上面我们看到的 `QueryError` , 有时候我们想要定义一个包含底层 `error` 的新 `error` 类型, 并将其保存以供代码检查. 这里 `QueryError` 再次登场:	

~~~go
type QueryError struct {
    Query string
    Err   error
}
~~~

程序可以检查 `*QueryError` 内部的值, 根据底层的 `error` 来做判断. 你有时候可以把这看作展开 `error` .

```go
if e, ok := err.(*QueryError); ok && e.Err == ErrPermission {
    // query failed because of a permission problem
}
```

#### Errors in Go 1.13

#### The Unwrap method

Go 1.13 介绍了  `errors` 和 `fmt` 标准库的新特性以简化对包含其他`errors` 的 `errors` 的处理. 其中最有效的是约定而不是更改:  一个包含其他`error` 的 `error` 可以实现一个 `Unwrap` 方法, 返回底层的 `error` . 如果 `e1.Unwrap()` 返回 `e2` , 那么我们称 `e1` 包装了 `e2` , 并且你可以展开 `e1` 以得到 `e2` .

```go
func (e *QueryError) Unwrap() error { return e.Err }
```

展开一个 `error` 的结果可能也有一个 `Unwrap` 方法; 我们可以调用从 `error` 链中展开的一系列 `errors`

#### Examining errors with Is and As

Go1.13 中 `errors` 包括了两个检验 `errors` 的新函数: `Is` 和 `As` .

The `errors.Is` 函数将 `error` 和一个 `value` 比较 .

```go
// Similar to:
//   if err == ErrNotFound { ... }
if errors.Is(err, ErrNotFound) {
    // something wasn't found
}
```

`As` 函数测试 `error` 是不是特定的类型 .

```go
// Similar to:
//   if e, ok :== err.(*QueryError); ok { ... }
var e *QueryError
if errors.As(err, &e) {
    // err is a *QueryError, and e is set to the error's value
}
```