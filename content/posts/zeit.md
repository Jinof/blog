---
title: "Use zeit to deploy hugo website"
date: 2020-03-26T18:32:01+08:00
draft: false
toc: false
images:
tags:
  - zeit
  - now
  - hugo
  - blog
---

## Bug
使用如下命令无法在zeit 中正确部署
```bash
cd themes;git clone -b master https://github.com/rhazdon/hugo-theme-hello-friend-ng;cd ../;hugo -D --gc 
```

报错如下
```
23:51:08.330  Error: No Output Directory named "public" found after the Build completed. You can configure the Output Directory in your project settings. Learn more: https://zeit.co/docs/v2/platform/frequently-asked-questions#missing-public-directory
```