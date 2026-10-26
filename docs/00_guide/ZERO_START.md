# v5.0 현재 PC에서 시작하기

[[SCOPE_v5]]와[[01_HARDWARE_BASELINE]]부터 읽습니다. 사용자의실제사양은Ryzen57530U/6C12T, RAM조회13.8GiB, C:최신실측여유25.98GiB, Windows11Home22631, WSL2 docker-desktop만존재입니다.

시작:S00~S05→S06/S09/S10/S19→M0/M1. JDK·DB·관리자는M2직전, Rust는M4직전, Go는배포의구체적필요시만설치합니다. S15와M7은범위제외입니다. 전체순서는[[03_EXECUTION_PLAN]]에있습니다.

초기WindowsPowerShell/C:\Dev\sideproject\My_Web기준입니다. 개발용Ubuntu는현재없으며Rust단계에서필요할때만추가여부를결정합니다. docker-desktop을개발용Ubuntu로착각하지않습니다. 기존파일·설정·DB를보존하고명령을한단계씩검증합니다.

본자료는실행전설계와지침입니다. 사용자PC에서설치·벤치마크를수행한것은아닙니다. 태스크별완료기록과실측결과를남겨다음AI에게인계합니다.

## v5.0 자료 워크플로 반영

초기에는 기존 최소 환경만 준비합니다. Python은 M8 직전 [[S20]]부터 설치합니다. [[SCOPE_v5]]
