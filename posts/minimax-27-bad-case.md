---
title: "Minimax 27 Bad Case"
date: 2026-03-29T08:44:50+08:00
draft: true
---

# 背景
在使用 Minimax 搭配 claude 的过程中, 发现其出现了错误

```python
"""
53. 最大子数组和
https://leetcode.cn/problems/maximum-subarray/

给你一个整数数组 nums，找到一个具有最大和的连续子数组（子数组最少包含一个元素），返回其最大和。
"""

from typing import List


def maxSubArray(nums: List[int]) -> int:
    if len(nums) == 0:
        return None
    min_prefix_sum = 0
    prefix_sum = nums[0]
    ans = nums[0]
    for i in range(1, len(nums)):
        prefix_sum += nums[i]
        if nums[i] > prefix_sum - min_prefix_sum:
            min_prefix_sum = 0
            ans = max(ans, nums[i])
        else:
            ans = max(ans, prefix_sum - min_prefix_sum)
        min_prefix_sum = min(min_prefix_sum, prefix_sum)
    return ans

# ============ 测试用例 ============

def test_max_sub_array():
    # 测试1: 基础用例
    result = maxSubArray([-2, 1, -3, 4, -1, 2, 1, -5, 4])
    print(f"测试1: {result}")
    assert result == 6

    # 测试2: 单元素
    result = maxSubArray([1])
    print(f"测试2: {result}")
    assert result == 1

    # 测试3: 全负
    result = maxSubArray([-1, -2, -3])
    print(f"测试3: {result}")
    assert result == -1

    # 测试4:
    result = maxSubArray([5,4,-1,7,8])
    print(f"测试4: {result}")
    assert result == 23

    # 测试5:
    result = maxSubArray([-2, 1])
    print(f"测试5: {result}")
    assert result == 1

    print("✅ 所有测试通过!")


if __name__ == "__main__":
    test_max_sub_array()
```
