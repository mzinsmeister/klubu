import org.jetbrains.kotlin.gradle.tasks.KotlinCompile
import org.gradle.nativeplatform.platform.internal.DefaultNativePlatform
import org.springframework.boot.gradle.tasks.bundling.BootJar

plugins {
	id("org.springframework.boot") version "2.7.3"
	id("io.spring.dependency-management") version "1.0.11.RELEASE"
	id("de.undercouch.download") version "4.1.2"
	kotlin("jvm") version "1.7.10"
	kotlin("plugin.spring") version "1.7.10"
	kotlin("plugin.jpa") version "1.7.10"
}

group = "dev.zinsmeister"
version = "0.0.1-SNAPSHOT"
java.sourceCompatibility = JavaVersion.VERSION_11

val kotestVersion = "5.4.2"

repositories {
	mavenCentral()
}

dependencies {
	implementation("org.springframework.boot:spring-boot-starter-data-jpa")
	//implementation("org.springframework.boot:spring-boot-starter-security")
	implementation("org.springframework.boot:spring-boot-starter-web")
	implementation("com.fasterxml.jackson.module:jackson-module-kotlin")

	implementation("com.googlecode.owasp-java-html-sanitizer:owasp-java-html-sanitizer:1.1")
	implementation("com.github.spullara.mustache.java:compiler:0.9.10")
	// New Docker baseimage should be built when a new playwright verson is for a new chromium version
	// TODO: Evaluate goging through chrome devtools protocol directly or using selenium instead
	implementation("org.seleniumhq.selenium:selenium-support:4.1.1")
	implementation("org.seleniumhq.selenium:selenium-chrome-driver:4.1.1")
	implementation("org.seleniumhq.selenium:selenium-remote-driver:4.1.1")
	implementation("org.seleniumhq.selenium:selenium-api:4.1.1")
	implementation("org.apache.pdfbox:pdfbox:2.0.24")
	implementation("org.apache.pdfbox:xmpbox:2.0.24")

	implementation("org.jetbrains.kotlin:kotlin-reflect")
	implementation("org.jetbrains.kotlin:kotlin-stdlib-jdk8")

	developmentOnly("org.springframework.boot:spring-boot-devtools")

	runtimeOnly("org.postgresql:postgresql")

	testImplementation("org.springframework.boot:spring-boot-starter-test")
	testImplementation("org.springframework.security:spring-security-test")
	testImplementation("io.mockk:mockk:1.12.2")
	testImplementation("io.kotest:kotest-runner-junit5:$kotestVersion")
	testImplementation("io.kotest:kotest-assertions-core:$kotestVersion")
	testImplementation("io.kotest:kotest-property:$kotestVersion")

	testRuntimeOnly("com.h2database:h2:2.0.206")
}

springBoot {
	mainClass.set("dev.zinsmeister.klubu.KlubuApplicationKt")
}

tasks.withType<KotlinCompile> {
	kotlinOptions {
		freeCompilerArgs = listOf("-Xjsr305=strict")
		jvmTarget = "11"
	}
}

tasks.withType<Test> {
	useJUnitPlatform()
}

val os = DefaultNativePlatform.getCurrentOperatingSystem()!!
val arch = DefaultNativePlatform.getCurrentArchitecture()!!

val chromiumBaseUrl="https://www.googleapis.com/download/storage/v1/b/chromium-browser-snapshots/o"
val chromiumVersion= if (os.isWindows && arch.isAmd64) {
	"938545"
} else if(os.isMacOsX && arch.isAmd64) {
	"938554"
} else if(os.isMacOsX && arch.isArm) {
	"938545"
} else if(os.isLinux && arch.isAmd64) {
	"938554"
} else {
	throw IllegalArgumentException("You are using an unsupported OS/Arch combination")
}

task<de.undercouch.gradle.tasks.download.Download>("downloadChromium") {
	if (os.isWindows && arch.isAmd64) {
		src("$chromiumBaseUrl/Win_x64%2F$chromiumVersion%2Fchrome-win.zip?alt=media")
	} else if(os.isMacOsX && arch.isAmd64) {
		src("$chromiumBaseUrl/Mac%2F$chromiumVersion%2Fchrome-mac.zip?alt=media")
	} else if(os.isMacOsX && arch.isArm) {
		src("$chromiumBaseUrl/Mac_Arm%2F$chromiumVersion%2Fchrome-mac.zip?alt=media")
	} else if(os.isLinux && arch.isAmd64) {
		src("$chromiumBaseUrl/Linux_x64%2F$chromiumVersion%2Fchrome-linux.zip?alt=media")
	} else {
		throw IllegalArgumentException("You are using an unsupported OS/Arch combination")
	}
	dest("$buildDir/chromium_download/chromium.zip")
	overwrite(false)
}

task<Copy>("unzipChromium") {
	dependsOn("downloadChromium")
	from(zipTree("$buildDir/chromium_download/chromium.zip"))
	into("$buildDir/chromium")
	eachFile {
		path = path.replaceFirst(Regex("^[^\\/]*/"), "")
	}
}

task<de.undercouch.gradle.tasks.download.Download>("downloadChromedriver") {
	if (os.isWindows && arch.isAmd64) {
		src("$chromiumBaseUrl/Win_x64%2F$chromiumVersion%2Fchromedriver_win32.zip?alt=media")
	} else if(os.isMacOsX && arch.isAmd64) {
		src("$chromiumBaseUrl/Mac%2F$chromiumVersion%2Fchromedriver_mac64.zip?alt=media")
	} else if(os.isMacOsX && arch.isArm) {
		src("$chromiumBaseUrl/Mac_Arm%2F$chromiumVersion%2Fchromedriver_mac64.zip?alt=media")
	} else if(os.isLinux && arch.isAmd64) {
		src("$chromiumBaseUrl/Linux_x64%2F$chromiumVersion%2Fchromedriver_linux64.zip?alt=media")
	} else {
		throw IllegalArgumentException("You are using an unsupported OS/Arch combination")
	}
	overwrite(false)
	dest("$buildDir/chromium_download/chromedriver.zip")
}

task<Copy>("unzipChromedriver") {
	dependsOn("downloadChromedriver")
	from(zipTree("$buildDir/chromium_download/chromedriver.zip"))
	into("$buildDir/chromium")
	eachFile {
		path = path.replaceFirst(Regex("^[^\\/]*/"), "")
	}
}

task<DefaultTask>("downloadDevExportBrowser") {
	dependsOn("unzipChromium", "unzipChromedriver")
}

tasks.withType<org.springframework.boot.gradle.tasks.run.BootRun> {
	if (os.isWindows && arch.isAmd64) {
		systemProperty("klubu.export.chromium.path", "./build/chromium/chrome.exe")
		systemProperty("klubu.export.chromedriver.path", "./build/chromium/chromedriver.exe")
	} else if(os.isMacOsX) {
		// I can't validate if this is actually correct
		systemProperty("klubu.export.chromium.path", "./build/chromium/Chromium.app/Contents/MacOS/Chromium")
		systemProperty("klubu.export.chromedriver.path", "./build/chromium/chromedriver")
	} else if(os.isLinux && arch.isAmd64) {
		systemProperty("klubu.export.chromium.path", "./build/chromium/chrome")
		systemProperty("klubu.export.chromedriver.path", "./build/chromium/chromedriver")

	} else {
		throw IllegalArgumentException("You are using an unsupported OS/Arch combination")
	}
}

//TODO: Build frontend and backend in one Gradle build

tasks.withType<BootJar> {
	exclude("static", "config")
	from("../frontend/dist") {
		into("static")
	}
}
