# scene: encounter

[SAY speaker=You]
Excuse me... where station go... I mean, where is the station?

[WAIT 1.5s]

[SAY speaker=You]
（しまった、めちゃくちゃな英語だったかも……）

[MODIFY name=affection op=+ value=20]

[SHOW_IMAGE name=city_crossing.png]
<!-- 街中の交差点。会話の舞台となる情景 -->

[PLAY_BGM name=street.mp3]

[WAIT 2s]

[SAY speaker=Foreigner]
Ahh... Eki wa... koko kara... migi?

[JUMP_IF affection>=15 label=helped]
<!-- 好感度が一定以上なら、親切な応答へ分岐 -->

[SAY speaker=You]
（やっぱり伝わらなかったかな……）

[JUMP label=end]

[LABEL name=helped]

[SAY speaker=You]
Thank you! I... appreciate your kindness!

[SHOW_TEXT_EFFECT effect=shake]

[BRANCH choice=Bow deeply label=bow choice=Smile quietly label=smile]
<!-- プレイヤー性格によって分かれる感謝表現。正解はない -->

[LABEL name=bow]
[SAY speaker=You]
ありがとう… very much...（深々と）

[JUMP label=end]

[LABEL name=smile]
[SAY speaker=You]
Thanks 😊

[LABEL name=end]

[SAY speaker=Foreigner]
気をつけて〜！

[EMIT_EVENT name=ending_flag]