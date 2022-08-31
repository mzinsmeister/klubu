package dev.zinsmeister.klubu.receipt.init

import dev.zinsmeister.klubu.receipt.domain.ReceiptItemCategory
import dev.zinsmeister.klubu.receipt.domain.ReceiptItemCategoryType
import dev.zinsmeister.klubu.receipt.repository.ReceiptItemCategoryRepository
import dev.zinsmeister.klubu.receipt.repository.ReceiptItemCategoryTypeRepository
import org.slf4j.LoggerFactory
import org.springframework.beans.factory.annotation.Value
import org.springframework.boot.CommandLineRunner
import org.springframework.stereotype.Component
import org.springframework.transaction.annotation.Transactional

@Component
class ReceiptItemCategoryInitRunner(
    @Value("#{'\${klubu.receipt.categories.defaultCategoryTypes:}'.split(';')}") private val defaultCategoryTypes: List<String>,
    private val itemCategoryRepository: ReceiptItemCategoryRepository,
    private val itemCategoryTypeRepository: ReceiptItemCategoryTypeRepository
    ): CommandLineRunner {

    val logger = LoggerFactory.getLogger(this::class.java)

    @Transactional
    override fun run(vararg args: String?) {
        logger.info("Creating missing default receipt item category types")
        val itemCategoryTypesIterator = itemCategoryTypeRepository.findAll().map { it.name }.sorted().iterator()
        val sortedDefaultCategories = defaultCategoryTypes.sorted()
        var currentCategoryType = if(itemCategoryTypesIterator.hasNext()) {itemCategoryTypesIterator.next()} else {null}
        for(type in sortedDefaultCategories) {
            if(currentCategoryType == null || type < currentCategoryType) {
                var currentDefaultCategoryTypeEntity = ReceiptItemCategoryType(type)
                currentDefaultCategoryTypeEntity = itemCategoryTypeRepository.save(currentDefaultCategoryTypeEntity)
                val currentDefaultCategoryEntity = ReceiptItemCategory(type, currentDefaultCategoryTypeEntity)
                itemCategoryRepository.save(currentDefaultCategoryEntity)
                logger.info("Created missing default category type $type and a default category for it")
                currentCategoryType = if(itemCategoryTypesIterator.hasNext()) {itemCategoryTypesIterator.next()} else {null}
            }
            if(type == currentCategoryType) {
                logger.info("Skipping default category type $type because it already exists")
            }
        }
    }
}