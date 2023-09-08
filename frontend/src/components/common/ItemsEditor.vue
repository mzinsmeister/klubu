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
        <tr v-for="(item, i) in modelValue" :key="i">
          <td :value="i + 1" />
          <td>
            <o-input
              @input="change"
              :disabled="isDisabled"
              class="position-input"
              v-model="item.item"
            />
          </td>
          <td>
            <o-input
              @input="change"
              :disabled="isDisabled"
              class="position-input"
              style="max-width: 50px"
              v-model="item.quantity"
            />
          </td>
          <td>
            <o-input
              @input="change"
              :disabled="isDisabled"
              class="position-input"
              style="max-width: 80px"
              v-model="item.unit"
            />
          </td>
          <td>
            <o-input
              @input="change"
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
              type="is-danger"
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

import { computed } from "vue";
import { formatCentsAsMoney } from "@/util/MoneyUtil";
import { type Item } from "@/models/CommonModel";



let { modelValue, disabled } = defineProps<{
    modelValue:  Item[], 
    disabled?: boolean
  }>();

const emit = defineEmits(["change", "update:modelValue"]);

  const created = ()  => {
    if (modelValue.length === 0) {
      addEmptyItem();
    }
  }
  const isDisabled = computed((): boolean => {
    return disabled !== undefined ? disabled : false;
  });
  const deleteItem = (index: number)  => {
    emit("update:modelValue", modelValue.filter((_: any, i: number) => i !== index));
    change();
  }

  const addEmptyItem = (): void => {
    const newItem: Item = {
      item: "",
      quantity: 1,
      unit: "",
      price: { amountCents: 0, currency: { code: "EUR" } },
    };
    emit("update:modelValue", modelValue.concat(newItem));
    change();
  }
  const change = (): void => {
    emit("change");
  }
  void created();
</script>
<style scoped lang="scss"></style>