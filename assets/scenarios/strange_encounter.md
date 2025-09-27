# scene: opening
<!-- BGM: 穏やかな朝のテーマ -->
[PLAY_BGM name=intro]

<!-- Ayumi が少し照れている表情を表示 -->
[SHOW_IMAGE file=ayumi_happy]

[SAY speaker=Ayumi]
やっと着いたね。

<!-- 方向選択。右は明るい道、左はショートカット -->
[BRANCH choice=右へ label=go_right, choice=左へ label=go_left]

[LABEL name=go_right]
[SAY speaker=Ayumi]
こっちは遠回りだけど景色がいいよ。

[JUMP label=end]

[LABEL name=go_left]
[SAY speaker=Ayumi]
こっちは近道。ちょっと暗いけどね。

[LABEL name=end]
[SAY speaker=Ayumi]
また来ようね。