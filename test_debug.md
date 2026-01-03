# scene: opening

[SAY speaker=Narrator]
Welcome to the debug test scenario.

[SAY speaker=Ayumi]
Hello! I'm Ayumi.

[SET name=score value=10]

[SAY speaker=System]
Score has been set to 10.

[BRANCH choice=Increase score label=increase, choice=Show score label=show]

# scene: increase
[LABEL name=increase]

[MODIFY name=score op=add value=5]

[SAY speaker=System]
Score increased by 5.

[JUMP label=end]

# scene: show
[LABEL name=show]

[SAY speaker=System]
Current score is 10.

[JUMP label=end]

# scene: end
[LABEL name=end]

[SAY speaker=Narrator]
Thank you for testing!
