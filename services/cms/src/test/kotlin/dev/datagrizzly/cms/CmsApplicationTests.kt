package dev.datagrizzly.cms

import org.junit.jupiter.api.Test
import org.springframework.beans.factory.annotation.Autowired
import org.springframework.boot.webmvc.test.autoconfigure.AutoConfigureMockMvc
import org.springframework.boot.test.context.SpringBootTest
import org.springframework.test.web.servlet.MockMvc
import org.springframework.test.web.servlet.get

// 개념: CMS가 뜨는지와 health 공개 범위를 확인한다.
// 목적: health는 열고, 다른 actuator는 열지 않는다.
// 사용처: services/cms 의 gradlew.bat test.
@SpringBootTest
@AutoConfigureMockMvc
class CmsApplicationTests {

	@Autowired
	private lateinit var mockMvc: MockMvc

	@Test
	fun contextLoads() {
	}

	@Test
	fun healthIsUp() {
		mockMvc.get("/actuator/health")
			.andExpect {
				status { isOk() }
				jsonPath("$.status") { value("UP") }
			}
	}

	@Test
	fun envActuatorIsHidden() {
		mockMvc.get("/actuator/env").andExpect {
			status { isNotFound() }
		}
	}
}
