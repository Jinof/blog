---
title: "stack rpc里的registry"
date: 2020-11-23T00:04:42+08:00
draft: true
toc: false
images:
tags:
  - stack-rpc
---

version: < 1.00

首先说明, stack-rpc是基于go micro v1.18的微服务框架.
stack-rpc自己实现了一些registry, 例如 mdns, memory, etcd. 让我们看看他们的具体实现.

首先看看registry interface
```go
// The registry provides an interface for service discovery
// and an abstraction over varying implementations
// {consul, etcd, zookeeper, ...}
type Registry interface {
	Init(...Option) error
	Options() Options
	Register(*Service, ...RegisterOption) error  // Register a service node. Additionally supply options such as TTL.
	Deregister(*Service) error  // Deregister a service node
	GetService(string) ([]*Service, error)  // Retrieve a service. A slice is returned since we separate Name/Version.
	ListServices() ([]*Service, error)  // List the services. Only returns service names.
	Watch(...WatchOption) (Watcher, error)  // Watch returns a watcher which allows you to track updates to the registry.
	String() string  // String return the registry name.
}
```
registry里最重要的是Register

