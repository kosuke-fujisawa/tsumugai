# Test Scenario for GOTO / Conditional Expressions / Ending

## Test: Scene with ending, GOTO, and conditional expressions

# scene: start

[SAY speaker=Narrator]
Welcome to the test scenario.

[SET name=score value=0]
[SET name=helped value=false]

[SAY speaker=Narrator]
You encounter a stranger in need.

[SAY speaker=Narrator]
Do you help them?

[SET name=helped value=true]
[MODIFY name=score op=add value=10]

:::when score > 5 && helped == "true"
[SAY speaker=Narrator]
Your kindness has been rewarded!

[MODIFY name=score op=add value=5]
:::

:::when score >= 15
[SAY speaker=Narrator]
You have achieved a high score!

[GOTO target=good_end]
:::

:::when score < 15
[SAY speaker=Narrator]
Your score is not high enough...

[GOTO target=bad_end]
:::

# scene: good_end
@ending GOOD

[SAY speaker=Narrator]
Congratulations! You reached the good ending!

[LABEL name=end]

# scene: bad_end
@ending BAD

[SAY speaker=Narrator]
Unfortunately, this is a bad ending.

[GOTO target=end]
