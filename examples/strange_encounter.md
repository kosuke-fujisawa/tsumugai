# scene: street_corner

[PLAY_BGM name=street.mp3]

[SAY speaker=Stranger]
Excuse me... do you know where the station is?

[SAY speaker=Hero]
Ah... go straightly and turn right at the big tree.

[SAY speaker=Hero]
（しまった、"straightly" はおかしいかもしれない…）

[WAIT 1s]

[SAY speaker=Stranger]
Ahh... チガウ けど... ワカッタ、アリガト！

[WAIT 1s]

[SAY speaker=Hero]
（伝わった…？いや、たぶん伝わってない…）

[BRANCH choice=Try again label=retry, choice=Run away label=exit]

# scene: retry

[SAY speaker=Hero]
Ah... maybe... you can ask konbini.

[SAY speaker=Stranger]
Ohh! コンビニ！ヨク ワカッタ！

[WAIT 0.5s]

[SAY speaker=Hero]
（通じた…のか？）

# scene: exit

[SAY speaker=Hero]
（もうダメだ、走ろう…）

[WAIT 1s]

[SAY speaker=Stranger]
ナゼ ニゲル！？ ワタシ コワクナイ！