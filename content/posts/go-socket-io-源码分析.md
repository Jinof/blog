---
title: "go-socket.io 源码分析"
date: 2020-06-07T23:53:22+08:00
draft: true
toc: false
images:
tags:
  - go
  - socket.io
  - websocket
---

go-socket.io 的源码很简单，读起来其实不费力（因为API很少）

先自顶向下看看我们启动一个 socket io server 都经过了哪些流程。

```go
import (
    "net/http"
	engineio "github.com/googollee/go-engine.io"
)

// Server is a go-socket.io server.
type Server struct {
	handlers map[string]*namespaceHandler
	eio      *engineio.Server
}

// NewServer returns a server.
func NewServer(c *engineio.Options) (*Server, error) {
	eio, err := engineio.NewServer(c)
	if err != nil {
		return nil, err
	}
	return &Server{
		handlers: make(map[string]*namespaceHandler),
		eio:      eio,
	}, nil
}
```

NewServer 首先调用 go-engine.io 中的 NewServer， 将得到的 eio 赋值给了 Server{}，并返回该 Server 的指针。（这才是代码少真正的原因，很多实现都是在 go-engine.io 库里）

这里我们仅关注 Server 中的 handlers，毕竟 eio 是另一个库的。

handlers 的数据结构为 map[string]*namespaceHandler

~~~go
type namespaceHandler struct {
	onConnect    func(c Conn) error
	onDisconnect func(c Conn, msg string)
	onError      func(c Conn, err error)
	events       map[string]*funcHandler
	broadcast    Broadcast
}
~~~

上面是 namespaceHandler 的定义，这其实就是一个实现了 go-socketio 中各个方法的结构体。

那么 handlers 是用来干嘛的呢？

以 onConnect 为例子

~~~go
// OnConnect set a handler function f to handle open event for
// namespace nsp.
func (s *Server) OnConnect(nsp string, f func(Conn) error) {
	h := s.getNamespace(nsp, true)
	h.OnConnect(f)
}
~~~

onConnect 先获取 namespace 然后执行 h.OnConnect()

~~~go
func (s *Server) getNamespace(nsp string, create bool) *namespaceHandler {
	if nsp == "/" {
		nsp = ""
	}
	ret, ok := s.handlers[nsp]
	if ok {
		return ret
	}
	if create {
		handler := newHandler()
		s.handlers[nsp] = handler
		return handler
	} else {
		return nil
	}
}
~~~

getNamespace 接受两个参数 nsp 和 create, 当 nsp 为 "/" 会被重写为 ""。然后判 handlers 该 nsp 是否存在，存在就直接返回。再判断是否 create，如果为 true 则创建一个 handler 再将 nsp 和改 handler 加入 handlers 中。否则返回 nil。

```go
func newHandler() *namespaceHandler {
	return &namespaceHandler{
		events:    make(map[string]*funcHandler),
		broadcast: NewBroadcast(),
	}
}

func (h *namespaceHandler) OnDisconnect(f func(Conn, string)) {
	h.onDisconnect = f
}
```

从上面看出 newHandler() 实际上就是创建一个 namespaceHandler 。而 OnDisconnect 就是将  namespaceHandler 复制给 namespaceHandler 中的 onDisconnect 字段。

~~~go
func (h *namespaceHandler) OnDisconnect(f func(Conn, string)) {
	h.onDisconnect = f
}

func (h *namespaceHandler) OnError(f func(Conn, error)) {
	h.onError = f
}
~~~

OnDisconnect 和 OnError 与 onConnect 是同样的实现方法。