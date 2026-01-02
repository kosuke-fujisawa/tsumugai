# scene: 出会い

[SHOW_IMAGE file=bg_street_morning]
[SHOW_IMAGE file=Ayumi_normal]
[PLAY_MUSIC file=intro]
[POSITION Ayumi=left]

[SAY speaker=Ayumi]
こんにちは。[c]
tsumugaiのデモにようこそ！[c]

[WAIT 1.0s]

# scene: opening
[SHOW_IMAGE file=ayumi_neutral]

[SAY speaker=Ayumi]
やっと着いたね。ここからは、あなたが選ぶ番だよ。[c]

:::flag
helped_tourist: 旅行者を助けたかどうか
route_ayumi: あゆみルートに入った
:::

:::vars
help_count: 旅行者を助けた人数
:::

:::choices
- 駅へ急ぐ @station
- 旅行者を助ける @help
:::

:::route station
    [SAY speaker=Ayumi]
    困っているみたいだね。[c]
    :::choices
    - 声をかける @talk
    - 立ち去る @leave
    :::
    :::route talk
        [SET helped_tourist=TRUE]
        [SET help_count+=1]
        [SAY speaker=Protagonist]
        大丈夫ですか？[c]
    :::
:::

[WAIT 0.5s]

:::when helped_tourist=TRUE
[SAY speaker=Ayumi]
いいことをしたから、気分がいいな。[c]
:::

# scene: ending
<!-- Final line; engine should Halt after proceeding past last command -->
[SAY speaker=System]
Demo complete. （デモ終了。おつかれさま）
