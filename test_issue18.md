# scene: opening

[SHOW_IMAGE layer=bg name=school.png]
[PLAY_BGM name=morning.mp3]

[SAY speaker=Narrator]
Welcome to the test scenario for issue #18.

[SAY speaker=Alice]
Hello! This is a test.

[PLAY_SE name=bell.wav]

[SAY speaker=Bob]
Nice to meet you!

# scene: choice_test

[SAY speaker=Narrator]
Now, let's test the choice feature.

[BRANCH choice=Option A label=choice_a, choice=Option B label=choice_b]

[LABEL name=choice_a]

# scene: result_a

[SHOW_IMAGE layer=bg name=classroom_a.png]

[SAY speaker=Alice]
You chose Option A!

[JUMP label=end]

[LABEL name=choice_b]

# scene: result_b

[SHOW_IMAGE layer=bg name=classroom_b.png]

[SAY speaker=Bob]
You chose Option B!

[LABEL name=end]

[SAY speaker=Narrator]
Thank you for testing!
