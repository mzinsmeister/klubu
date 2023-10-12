# KLUBU - A small invoicing tool aimed at german business owners who fall under the small business rule (Kleinunternehmerregelung)

In my search for an open source invoicing tool for registering a small business as a side hustle i failed to find one which can comply with all the german regulations without significant modifications. So this is my (Work in progress) attempt at creating just that: A small invoicing and bookkeeping tool that does exactly what i need and nothing more or less for now. This is very much intended to be as simple and easy to understand as possible for the user as well as the programmer. 

The main regulation it is designed to comply with is the GoBD but since i'm neither a lawyer nor an accountant i won't be able to guarantee that it actually complies (but if lawyers or accountants wanted to make a contribution in the form of a legal review or sth i would be more than happy to take it). GDPR compliance is kinda also a goal even though i will make sure to actually comply at a later time (most of the stuff legally needs to be stored for a long long time (10 years) anyway).

## Features
Here's a list of features Klubu already has or is intended to have:

- Manage Contacts (Essentially Customer Contacts for now)
- Create offers with multiple revisions
- Create Invoices
- Manage Receipts
- Create Reports, most importantly the operating result with the net-income method needed for taxes
- Create beautiful PDF/A documents for Invoices and Offers with customizable templates

## Architecture
A basic overview of the architecutre is the following:

It's designed as a webapp that should later also be useable as a standalone Desktop App through Electron.

The frontend is in Typescript with Vue, the Backend is in Kotlin with Spring Boot. The Server version (the only one for 
now) is designed to be used with Postgres. The Desktopapp will use SQLite later. Spring Data JPA is used for 
Persistence/ORM. 

PDFs are rendered from HTML/CSS/JS with a Headless Chromium and paged.js through Selenium. This uses a lot of 
space and takes rather long to render compared to the alternatives like Flyingsaucer (which could still rather 
easily be swapped in) but imo that tradeoff is worth it because this way you'll be able to use a modern 
HTML/CSS renderer and you'll be able to use JS with probably the most advanced JS engine out there (V8) and with 
paged.js you'll be able to use basically all of the CSS paged media standards even those which are still working 
drafts. Mustache is used for creating HTML Document templates. Since for now the software is basically intended 
to be used by a single (obviously not mallicious) person, the HTML inserted into the documents by the client 
is assumed not to be mallicious but a TODO would be to sanitize HTML like that using the OWASP HTML Sanitizer 
for Java. Since Documents need to be in PDF/A Format for GoBD compliance the generated PDF documents are then 
converted to PDF/A with the Apache PDFBox library. The european standard for electronic invoices 
ZUGFeRD/Factur-X which is also based on PDF/A is also a nice to have for the future which could be achived using 
the Mustang library.

For now there is no access conrol as I want to have a fully functional prototype first. As long as it's not part 
of the application, HTTP basic auth or something else could be used by using a reverse proxy in front of the 
backend and frontend. But building actual authentication into the application itself is a high priority TODO right
after fininshing the core functinality. It will then be done using Spring Security.

There's also currently no setup for the main organisation/freelancer data (name, address, payment details, ...). 
This data, for now, is configured in a config file on the server. This is obviously not ideal but it will do for now.

## Licence
For now KluBu is available under the GNU Affero Gneral Public Licence (AGPL). This is mostly a default to one of
the most restrictive licences to give me more time to think about the lincence I actually want to use long term.

## Development
You will need a JDK 17 or higher and npm (latest lts should work). For development of the webapp use
```    
npm run dev
```
and for development of the Kotlin backend use
```
./gradlew bootRun
```
To quickly start a development database, you can use the docker-compose.yml file inside the backend directory.

## Installation
Linux is the only inteded environment for production. Theoretically windows should also work 
(it does work for development) but might take some fiddling. A Docker image can be built and used 
(Is currently not available in any docker repository but might be uploaded to DockerHub once the project is more 
stable) with the Dockerfile in the root directory of the repository. Otherwise if you really want to you can run 
it without docker but I don't think this kind of deployment will ever be oficially supported.You will need to install 
Chrom(ium/e) and a chrom(ium/e)driver that works with the installed version of Chrom(ium/e). Just try installing 
Chromium and the driver through the package manager of your distribution. You will then also need a JRE or JDK 11 
or higher, try something like Eclipse Temurin or alternatively use whatever OpenJDK variant your package manager 
provides you with. Then you will need to build the webapp by calling
```
npm run build
```
inside the frontend directory (make sure you have npm installed). After that build the FatJar with
```
./gradlew bootJar
```
which should download the correct version of Gradle aswell as all dependencies and build the jar package 
inside the backend/build/libs directory as klubu-[version].jar. You can then run that with
```
java -jar klubu-[version].jar
```
For configuration you can either create a config directory and then look at the application-dev.properties 
and the other config files  configuration and create an application-prod.properties file or you can use 
environment variables or system properties using the same properies used in that file. Then all that's 
left to do is installing a Postgres database (ideally through docker too) and configuring the connection 
to that and creating a templates folder and mounting that into the container and putting your personal/company 
information into a user.properties file or using environment variables for that too (this config file is more 
of a hack btw.). You can have a look at the Spring Boot docs for all the different ways you can use for 
configuration. In general look at the development config under backend/main/resources/config for reference.

If you quickly want to have an instance up and running for testing purposes there's a docker-compose.yml 
file in the root directory that allows you to do that. Just copy the user.properties file in backend/main/resources/config
to a config directory in the root of the project and call 
```    
docker compose up
```
in the root directory and a docker image should be built and run in combination with a postgres database.

For production use, you can create a docker-compose.yml file similar to that one. In general the whole installation
experience still needs some work.

## Database

Liquibase is integrated now. However it's not sure whether this actually works properly in all cases. This will allow KluBu to
be used with almost any RDBMS but only Postgres (and SQLite once (and if) a standalone Desktop version is introduced)
will be actually tested with for now and unless you have a very good reason not to, just use that.
Since the tool is currently (and probably will always be) only meant for people who don't have a lot of invoices, 
offers, receipts and contacts anyway, optimizing the database by creating indexes is currently not a priority 
since a lot of accesses happen through the primary key anyway or will result in sequential scans anyway and most 
table will stay in the ballpark of a few thousand rows at most, there will be almost no performance difference
between sequential scans and index accesses anyway.
