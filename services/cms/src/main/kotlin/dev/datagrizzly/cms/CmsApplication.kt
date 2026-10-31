package dev.datagrizzly.cms

import org.springframework.boot.autoconfigure.SpringBootApplication
import org.springframework.boot.runApplication

// 개념: CMS API의 시작점.
// 목적: 웹, 검증, health만 올린다. 데이터베이스와 인증은 아직 없다.
// 사용처: bootJar로 만든 cms JAR의 main.
@SpringBootApplication
class CmsApplication

fun main(args: Array<String>) {
	runApplication<CmsApplication>(*args)
}
