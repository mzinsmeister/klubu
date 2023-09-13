<template>
  <div class="recipent-editor">
    <o-field grouped>
      <o-field expanded label="Anrede">
        <o-input
          :disabled="isDisabled"
          :modelValue="modelValue.formOfAddress"
          @update:modelValue="$emit('update:modelValue', { ...modelValue, formOfAddress: $event }); change()"
        />
      </o-field>
      <o-field expanded label="Titel">
        <o-input 
          :disabled="isDisabled" 
          :modelValue="modelValue.title"
          @update:modelValue="$emit('update:modelValue', { ...modelValue, title: $event }); change()" />
      </o-field>
    </o-field>
    <o-field label="Name">
      <o-input
        :disabled="isDisabled"
        expanded
        :modelValue="modelValue.firstName"
        @update:modelValue="$emit('update:modelValue', { ...modelValue, firstName: $event }); change()"
      />
      <o-input
        :disabled="isDisabled"
        expanded
        :modelValue="modelValue.name"
        @update:modelValue="$emit('update:modelValue', { ...modelValue, name: $event }); change()"
      />
    </o-field>
    <o-field grouped>
      <o-field expanded label="Straße">
        <o-input
          :disabled="isDisabled"
          :modelValue="modelValue.street"
          @update:modelValue="$emit('update:modelValue', { ...modelValue, street: $event }); change()"
        />
      </o-field>
      <o-field expanded label="Hausnummer">
        <o-input
          :disabled="isDisabled"
          :modelValue="modelValue.houseNumber"
          @update:modelValue="$emit('update:modelValue', { ...modelValue, houseNumber: $event }); change()"
        />
      </o-field>
    </o-field>
    <o-field grouped>
      <o-field expanded label="PLZ">
        <o-input
          :disabled="isDisabled"
          :modelValue="modelValue.zipCode"
          @update:modelValue="$emit('update:modelValue', { ...modelValue, zipCode: $event }); change()"
        />
      </o-field>
      <o-field expanded label="Ort">
        <o-input 
          :disabled="isDisabled" 
          :modelValue="modelValue.city"
          @update:modelValue="$emit('update:modelValue', { ...modelValue, city: $event }); change()" 
        />
      </o-field>
    </o-field>
    <o-field label="Land">
      <o-input 
        :disabled="isDisabled"
        :modelValue="modelValue.country"
        @update:modelValue="$emit('update:modelValue', { ...modelValue, country: $event }); change()"
      />
    </o-field>
  </div>
</template>

<script setup lang="ts">

import { computed } from "vue";
import { type Recipient } from "@/models/CommonModel";

const props = defineProps<{
  disabled?: boolean,
  modelValue:  Recipient,
}>()

const emit = defineEmits(["change", "update:modelValue"]);

const isDisabled = computed((): boolean => {
  return props.disabled !== undefined ? props.disabled : false;
});
const change = (): void => {
  emit("change");
}
</script>
<style scoped lang="scss"></style>