<template>
  <div class="recipent-editor">
    <o-field grouped>
      <o-field expanded label="Anrede">
        <o-input
          @input="change"
          :disabled="isDisabled"
          :modelValue="modelValue.formOfAddress"
          @update:modelValue="$emit('update:modelValue', { ...modelValue, formOfAddress: $event })"
        />
      </o-field>
      <o-field expanded label="Titel">
        <o-input 
          @input="change" 
          :disabled="isDisabled" 
          :modelValue="modelValue.title"
          @update:modelValue="$emit('update:modelValue', { ...modelValue, title: $event })" />
      </o-field>
    </o-field>
    <o-field label="Name">
      <o-input
        @input="change"
        :disabled="isDisabled"
        expanded
        :modelValue="modelValue.firstName"
        @update:modelValue="$emit('update:modelValue', { ...modelValue, firstName: $event })"
      />
      <o-input
        @input="change"
        :disabled="isDisabled"
        expanded
        :modelValue="modelValue.name"
        @update:modelValue="$emit('update:modelValue', { ...modelValue, name: $event })"
      />
    </o-field>
    <o-field grouped>
      <o-field expanded label="Straße">
        <o-input
          @input="change"
          :disabled="isDisabled"
          :modelValue="modelValue.street"
          @update:modelValue="$emit('update:modelValue', { ...modelValue, street: $event })"
        />
      </o-field>
      <o-field expanded label="Hausnummer">
        <o-input
          @input="change"
          :disabled="isDisabled"
          :modelValue="modelValue.houseNumber"
          @update:modelValue="$emit('update:modelValue', { ...modelValue, houseNumber: $event })"
        />
      </o-field>
    </o-field>
    <o-field grouped>
      <o-field expanded label="PLZ">
        <o-input
          @input="change"
          :disabled="isDisabled"
          :modelValue="modelValue.zipCode"
          @update:modelValue="$emit('update:modelValue', { ...modelValue, zipCode: $event })"
        />
      </o-field>
      <o-field expanded label="Ort">
        <o-input 
          @input="change"
          :disabled="isDisabled" 
          :modelValue="modelValue.city"
          @update:modelValue="$emit('update:modelValue', { ...modelValue, city: $event })" 
        />
      </o-field>
    </o-field>
    <o-field label="Land">
      <o-input 
        @input="change"
        :disabled="isDisabled"
        :modelValue="modelValue.country"
        @update:modelValue="$emit('update:modelValue', { ...modelValue, country: $event })"
      />
    </o-field>
  </div>
</template>

<script setup lang="ts">

import { computed } from "vue";
import { type Recipient } from "@/models/CommonModel";

const { disabled, modelValue } = defineProps<{
  disabled?: boolean,
  modelValue:  Recipient,
}>()

const emit = defineEmits(["change", "update:modelValue"]);

const isDisabled = computed((): boolean => {
  return disabled !== undefined ? disabled : false;
});
const change = (): void => {
  emit("change");
}
</script>
<style scoped lang="scss"></style>