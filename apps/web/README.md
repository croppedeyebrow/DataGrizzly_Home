# @datagrizzly/web

DataGrizzly 공개 웹입니다. 의존성 설치와 실행은 저장소 루트에서 Yarn workspace로만 합니다.

```powershell
corepack yarn install --immutable
corepack yarn workspace @datagrizzly/web dev --host 127.0.0.1 --port 4321
corepack yarn workspace @datagrizzly/web check
corepack yarn workspace @datagrizzly/web build
```

개발 서버 주소는 `astro.config.mjs`의 `127.0.0.1:4321`이다. 확인된 절차는 루트 `docs/setup/local-run.md`에 있다.

이 폴더에서 따로 설치하거나 Git을 초기화하지 않습니다. 잠금 파일은 루트 `yarn.lock` 하나만 유지합니다.
