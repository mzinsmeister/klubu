# KLUBU - A small invoicing tool aimed at german business owners wich fall under the small business rule (Kleinunternehmerregelung)

In my search for an open source invoicing tool for registering a small business as a side hustle i failed to find one which can comply with all the german regulations without significant modifications. So this is my (Work in progress) attemt at creating just that: A small invoicing and bookkeeping tool that does exactly what i need and nothing more or less for now. This is very much intended to be as simple and easy to understand as possible for the user as well as the programmer. 

The main regulation it is designed to comply with is the GoBD but since i'm neither a lawyer nor an accountant i won't be able to guarantee that it actually complies (but if lawyers of accountants wanted to make a contribution in the form of a legal review or sth i would be more than happy to take it). GDPR compliance is kinda also a goal even though i will make sure to actually comply at a later time.

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

The frontend is in Typescript with Vue, the Backend is in Kotlin with Spring Boot. The Server version (the only one for now) is designed to be used with Postgres. The Desktopapp will use SQLite later. Spring Data JPA is used for Persistence/ORM. 

PDFs are rendered from HTML/CSS/JS with a Headless Chromium and paged.js through Playwright. This uses a lot of space and takes rather long to render compared to the alternatives like Flyingsaucer (which could still rather easily be swapped in) but imo that tradeoff is worth it because this way you'll be able to use a modern HTML/CSS renderer and you'll be able to use JS with probably the most advanced JS engine out there (V8) and with paged.js you'll be able to use basically all of the CSS paged media standards even those which are still working drafts. Mustache is be used for creating HTML Document templates. Since for now the software is basically intended to be used by a single (obviously not mallicious) person. That's why HTML inserted into the documents by the client won't be mallicious but a TODO would be to sanitize HTML like that with the OWASP HTML Sanitizer for Java. Since Documents need to be in PDF/A Format for GoBD compliance the generated PDF documents are then converted to PDF/A with the Apache PDFBox library. The european standard for electronic invoices ZUGFeRD/Factur-X which is also based on PDF/A is also a nice to have for the future which could be achived using the Mustang library.

For now there is no access conrol as i want to have a fully functional prototype first. As long as it's not part of the application, HTTP basic auth or something else could be used by using a reverse proxy in front of the backend and frontend. But building actual authentication into the application itself is a high priority TODO right after fininshing the core functinality. It will then be achived by using Spring Security.

# Installation
Installation is intended to be done with Docker on the Server but this is also still a TODO. The image will then include the Web Frontend, the JVM Backend aswell as the chromium browser for generating PDFs so that you will only need to start a database and the Klubu Container