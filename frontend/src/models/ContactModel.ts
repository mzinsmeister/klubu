export interface Contact {
  id?: number;
  formOfAddress?: string;
  title?: string;
  name: string;
  firstName?: string;
  street?: string;
  zipCode?: string;
  city?: string;
  houseNumber?: string;
  country?: string;
  phone?: string;
  isPerson: boolean;
}