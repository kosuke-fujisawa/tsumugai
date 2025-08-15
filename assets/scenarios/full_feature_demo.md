# scene: title
<!-- Title scene: quiet ambience, logo fade-in -->
[PLAY_BGM name=intro]
[SHOW_IMAGE file=logo_main]
[SAY speaker=System]
Welcome to **Tsumugai** demo.  
（ようこそ。Enter で進みます）

[WAIT 1.0s]

# scene: opening
<!-- Opening: street background, heroine appears -->
[SHOW_IMAGE file=bg_street_morning]
[SHOW_IMAGE file=ayumi_neutral]

[SAY speaker=Ayumi]
やっと着いたね。ここからは、あなたが選ぶ番だよ。

<!-- Offer a quick fork to exercise Branch handling -->
[BRANCH choice=Help the lost tourist (英語) label=help_tourist, choice=Head to the station (駅へ急ぐ) label=go_station]

# scene: help_tourist
<!-- SE plays non-blocking; SAY will block next -->
[PLAY_SE name=ui_select]
[SAY speaker=Protagonist]
Excuse me, do you need help? (えっと……Can I help you?)

<!-- Tourist replies in broken Japanese to test multilingual text -->
[SAY speaker=Tourist]
Ah… エキ、どこ？ ワタシ、まよった。

<!-- Increase affinity when being kind -->
[SET name=affinity value=0]
[MODIFY name=affinity op=add value=3]

[SAY speaker=Protagonist]
Go straight and turn left at the bakery. パン屋の角を左ですよ。

[WAIT 0.5s]
[PLAY_SE name=se_small_success]

[SAY speaker=Tourist]
アリガト！ You very kind!

[JUMP label=checkpoint]

# scene: go_station
<!-- Rushed route: less affinity -->
[SET name=affinity value=0]
[MODIFY name=affinity op=add value=1]
[PLAY_SE name=footsteps_fast]
[SAY speaker=Protagonist]
We’re running late… 急ごう。

[JUMP label=checkpoint]

# scene: checkpoint
<!-- Checkpoint label to test Jump/Label notifications -->
[SAY speaker=Ayumi]
さて、どうする？ ここからもう一択いこうか。

[BRANCH choice=Buy coffee (コーヒー) label=coffee, choice=Skip (スキップ) label=skip]

# scene: coffee
[PLAY_SE name=coin_drop]
[SAY speaker=Protagonist]
Two coffees, please. （温かいの、2つ）

[MODIFY name=affinity op=add value=2]
[JUMP label=after_break]

# scene: skip
[SAY speaker=Protagonist]
We’ll pass. （今はやめとこう）
[JUMP label=after_break]

# scene: after_break
<!-- Inline SAY variant on one line -->
[SAY speaker=Ayumi] そういえば、さっきの対応、けっこう良かったかも。

<!-- Simple movie cue to exercise WAIT-type other than SAY -->
[PLAY_MOVIE file=intro_cutscene]

<!-- Branch on variable using JumpIf -->
[JUMP_IF var=affinity cmp=ge value=4 label=good_route]
[JUMP label=normal_route]

# scene: good_route
[PLAY_BGM name=theme_warm]
[SAY speaker=Ayumi]
今日はうまくいきそう。Thank you.

[SHOW_IMAGE file=ayumi_smile]
[WAIT 0.8s]
[SAY speaker=Protagonist]
Let’s keep going.

[JUMP label=ending]

# scene: normal_route
[PLAY_BGM name=theme_neutral]
[SAY speaker=Ayumi]
まあ、ぼちぼち、かな。It’s fine.

[SHOW_IMAGE file=ayumi_neutral]
[WAIT 0.8s]
[SAY speaker=Protagonist]
We’ll figure it out.

[JUMP label=ending]

# scene: ending
<!-- Final line; engine should Halt after proceeding past last command -->
[SAY speaker=System]
Demo complete. （デモ終了。おつかれさま）
