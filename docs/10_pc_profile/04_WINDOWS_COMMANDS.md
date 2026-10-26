# Windows 개발 실행·자원 설정

기본 터미널은 PowerShell입니다. 예시 실행 위치는 `C:\Dev\sideproject\My_Web`입니다. 기존 설치 버전을 먼저 조회하고, 프로젝트 패키지 관리에는 Corepack으로 고정된 Yarn을 사용합니다.

## 읽기 전용 확인

```powershell
Get-Command git,node,npm,corepack,java,docker,go -ErrorAction SilentlyContinue
Get-PSDrive C | Select-Object Name,@{Name="FreeGiB";Expression={[math]::Round($_.Free/1GB,1)}}
Get-CimInstance Win32_OperatingSystem | Select-Object Caption,BuildNumber,@{Name="FreeRAMGiB";Expression={[math]::Round($_.FreePhysicalMemory/1MB,1)}}
docker system df
docker ps --format "table {{.Names}}\t{{.Status}}"
wsl -l -v
```

## 작업 폴더와 웹

```powershell
New-Item -ItemType Directory -Force C:\Dev\sideproject\My_Web
Set-Location C:\Dev\sideproject\My_Web
# 의도적인 의존성 변경 후 잠금 파일 갱신
corepack yarn install
# 평상시 재현 설치
corepack yarn install --immutable
corepack yarn workspace @datagrizzly/web dev --host 127.0.0.1 --port 4321
# 서버가 실행 중이면 아래 빌드는 다른 작업으로 분리. 평소에는 watcher를 종료하고 실행
# corepack yarn workspace @datagrizzly/web build
```

루트 workspace에는 처음에 `apps/web`만 등록합니다. S11에서 `apps/admin`을 추가하고 `packages/*`는 실제 package.json 생성 시 등록합니다. 정확한 `yarn.lock` 하나만 유지합니다. `.yarnrc.yml`의 설치 스크립트 차단을 임의로 풀지 않고, 필요한 패키지만 검토 후 개별 허용합니다.

## Kotlin

services/cms/gradle.properties 시작 예시:

```properties
org.gradle.jvmargs=-Xmx1024m -Dfile.encoding=UTF-8
org.gradle.workers.max=2
org.gradle.parallel=false
kotlin.compiler.execution.strategy=in-process
```

```powershell
Set-Location C:\Dev\sideproject\My_Web\services\cms
.\gradlew.bat test --max-workers=2
.\gradlew.bat bootJar --max-workers=2
.\gradlew.bat --stop
# 빌드된 실제 실행 JAR 이름으로 교체; plain.jar는 실행 대상 아님
# java -Xms128m -Xmx768m -jar .\build\libs\<실제bootJar>.jar --spring.profiles.active=local
```

bootRun을 쓸 때 앱 JVM heap은 Gradle heap과 별개이므로 BootRun의 jvmArgs를 설정합니다. 빌드 후 bootJar 실행은 Gradle daemon을 끄고 API만 실행하는 선택 경로입니다. test maxParallelForks=1과 테스트 JVM heap도 build.gradle.kts에서 별도로 설정합니다.

## PostgreSQL local 예시

기존 compose db 서비스에 아래 키를 병합합니다. DB 이미지와 영속볼륨은 기존 S13 정책대로 유지합니다.

```yaml
mem_limit: 768m
cpus: 1.0
command: ["postgres", "-c", "shared_buffers=128MB", "-c", "max_connections=20"]
```

```powershell
Set-Location C:\Dev\sideproject\My_Web
docker compose --env-file .env -f infra/local/compose.yaml up -d db
docker stats --no-stream
docker compose --env-file .env -f infra/local/compose.yaml stop db
```

## WSL 제한 — 선택 적용

%UserProfile%\.wslconfig의 기존 내용을 먼저 확인·백업한 뒤 병합합니다.

```ini
[wsl2]
memory=4GB
processors=4
swap=1GB
```

적용에는 WSL 재시작이 필요합니다. 실행 중인 Docker DB를 정상 중지하고 작업을 저장한 뒤 수행합니다. wsl --shutdown은 모든 WSL 배포판을 중단하므로 일반 조회 명령처럼 실행하지 않습니다. 설정 후 Task Manager와 docker stats로 결과를 확인합니다. 메모리 부족 시 먼저 동시 실행을 줄이고 그 뒤 상한을 재검토합니다.
