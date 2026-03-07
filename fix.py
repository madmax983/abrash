import re

content = open("src/experimental/kuwahara.rs").read()

# The error is: attempt to multiply with overflow
# At line 131: if total_variance_num * best_den < best_num * total_variance_den
# Since best_num starts at u64::MAX, `best_num * total_variance_den` will overflow if total_variance_den > 1!
# We can fix this by initializing best_num and best_den to something that won't overflow, or by doing the math with u128.
# total_variance_num is at most ~ 3 * (255^2) * count * count. (Wait, count can be up to 49, so count^2 is ~2500).
# u64::MAX is 18 * 10^18.
# We can just use u128 for the cross multiplication.
# Or we can just set best_num = u64::MAX and best_den = 1, and handle the first iteration.
# Or easier: u128::from(total_variance_num) * u128::from(best_den) < u128::from(best_num) * u128::from(total_variance_den)

content = content.replace("if total_variance_num * best_den < best_num * total_variance_den",
"if u128::from(total_variance_num) * u128::from(best_den) < u128::from(best_num) * u128::from(total_variance_den)")

open("src/experimental/kuwahara.rs", "w").write(content)
