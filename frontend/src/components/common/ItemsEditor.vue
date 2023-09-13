<template>
  <div class="items-editor">
    <table>
      <thead>
        <th></th>
        <th>Position</th>
        <th>Menge</th>
        <th>Einheit</th>
        <th>Einzelpreis (Cent)</th>
        <th>Betrag</th>
        <th></th>
      </thead>
      <tbody>
        <tr v-for="(item, i) in props.modelValue" :key="i">
          <td :value="i + 1" />
          <td>
            <o-input
              @update:modelValue="changeItem(i, {...item, item: $event})"
              :disabled="isDisabled"
              class="position-input"
              v-model="item.item"
            />
          </td>
          <td>
            <o-input
              @update:modelValue="changeItem(i, {...item, quantity: $event})"
              :disabled="isDisabled"
              class="position-input"
              style="max-width: 50px"
              v-model="item.quantity"
            />
          </td>
          <td>
            <o-input
              @update:modelValue="changeItem(i, {...item, unit: $event})"
              :disabled="isDisabled"
              class="position-input"
              style="max-width: 80px"
              v-model="item.unit"
            />
          </td>
          <td>
            <o-input
              @update:modelValue="changeItem(i, {...item, price: { ...item.price, amountCents: $event != '' ? Number.parseInt($event) : 0 }})"
              :disabled="isDisabled"
              class="position-input"
              style="max-width: 100px"
              v-model="item.price.amountCents"
            />
          </td>
          <td>
            {{ formatCentsAsMoney(item.quantity * item.price.amountCents) }}
          </td>
          <td>
            <o-button
              :disabled="isDisabled"
              icon-right="delete"
              variant="danger"
              @click="deleteItem(i)"
            />
          </td>
        </tr>
      </tbody>
    </table>
    <o-button @click="addEmptyItem()" :disabled="isDisabled"
      >Zusätzliche Position</o-button
    >
  </div>
</template>

<script setup lang="ts">

import { computed, onMounted } from "vue";
import { formatCentsAsMoney } from "@/util/MoneyUtil";
import { type Item } from "@/models/CommonModel";


const props = defineProps<{
    modelValue: Item[], 
    disabled?: boolean
  }>();


const emit = defineEmits(["change", "update:modelValue"]);

const isDisabled = computed((): boolean => {
  return props.disabled !== undefined ? props.disabled : false;
});

const deleteItem = (index: number)  => {
  emit("update:modelValue", props.modelValue.filter((_: any, i: number) => i !== index));
  change();
}

const changeItem = (i: number, item: Item) => {
  const newItems = [...props.modelValue];
  newItems[i] = item;
  emit("update:modelValue", newItems);
  change();
}

const addEmptyItem = (): void => {
  const newItem: Item = {
    item: "",
    quantity: 1,
    unit: "",
    price: { amountCents: 0, currency: { code: "EUR" } },
  };
  emit("update:modelValue", [...props.modelValue, newItem]);
  change();

}
const change = (): void => {
  emit("change");
}
onMounted(() => {
  if (props.modelValue.length === 0) {
    addEmptyItem();
  }
});

</script>
<style scoped lang="scss"></style>