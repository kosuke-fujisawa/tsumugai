# scene: デモシーン

@show_image bg:morning_sky
@play_bgm intro

Ayumi:
こんにちは。

Ayumi:
今日はいい天気だね。

@show_image character:ayumi_smile

Ayumi:
どこに行く?

@branch
1. 図書館へ行く -> route_library
2. 公園へ行く -> route_park

## route_library

# scene: 図書館

@show_image bg:library
@play_bgm calm

図書館は静かだった。

Ayumi:
ここで勉強しよう。

## route_park

# scene: 公園

@show_image bg:park
@play_bgm cheerful

公園には子供たちが遊んでいた。

Ayumi:
気持ちいいね！
