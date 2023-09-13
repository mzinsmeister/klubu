<template>
  <div class="items-editor">
    <div class="receipt-item-inputs" v-for="(item, i) in props.modelValue" :key="i">
      <o-field label="Beschreibung">
        <o-input
         @update:modelValue="change"
          :disabled="isDisabled"
          class="position-input"
          v-model="item.item"
        />
      </o-field>
      <o-field label="Kategorie">
        <o-select 
         @update:modelValue="change"
          :disabled="isDisabled"
          placeholder="Kategorie wählen"
          class="position-input"
          style="width: 100%;"
          v-model="item.category"
          :loading="item.category === undefined && itemCategories === null"
          >
            <option
                v-for="option in itemCategories"
                :value="option"
                :key="option.id">
                {{ option.name }}
            </option>
        </o-select>
      </o-field>
      <o-field grouped group-multiline>
        <o-field label="Preis in Cent">
          <o-input
            @update:modelValue="change(); item.price.amountCents = $event !== '' ? Number.parseInt($event) : 0"
            :disabled="isDisabled"
            class="position-input"
            v-model="item.price.amountCents"
          />
        </o-field>
        <o-field label="Löschen?">
          <o-button
            :disabled="isDisabled"
            icon-right="delete"
            variant="danger"
            @click="deleteItem(i)"
          />
      </o-field>
    </o-field>  
    </div>
    <o-button @click="addEmptyItem()" style="margin-top: 20px" :disabled="isDisabled"
      >Zusätzliche Position</o-button
    >
  </div>
</template>

<script setup lang="ts">

import { computed, ref, type Ref } from "vue";
import { fetchReceiptItemCategories } from "@/services/ReceiptsApiService";
import { type ReceiptItem, type ReceiptItemCategory } from "@/models/ReceiptModel";


const props = defineProps<{
  modelValue:  ReceiptItem[], 
  disabled?: boolean,
}>()

const emit = defineEmits(["change", "update:modelValue"]);
const itemCategories:  Ref<Array<ReceiptItemCategory> | null> = ref(null);

const change = (): void => {
  emit("change");
}

//TODO: amountCents is String after input but should be number (somehow still works)

const isDisabled = computed((): boolean => {
  return props.disabled !== undefined ? props.disabled : false;
});
const deleteItem = (index: number)  => {
  emit("update:modelValue", props.modelValue.filter((_: any, i: number) => i !== index));
  change();
}
const addEmptyItem = (): void => {
  const newItem: ReceiptItem = {
    item: "",
    price: { amountCents: 0, currency: { code: "EUR" } },
    category: undefined
  };
  emit("update:modelValue", [...props.modelValue, newItem]);
  change();
}
if (props.modelValue.length === 0) {
  addEmptyItem();
}
fetchReceiptItemCategories().then((data) => {
  itemCategories.value = data;
})
</script>
<style scoped lang="scss">
  .receipt-item-inputs {
    margin-top: 20px;
    border-bottom: 1px solid lightgrey;
    padding-bottom: 20px;
  }
</style>