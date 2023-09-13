import { createRouter, createWebHashHistory } from 'vue-router'
import Contacts from "../views/ContactsView.vue";
import Home from "../views/HomeView.vue";
import Invoices from "../views/InvoicesView.vue";
import InvoicesOverview from "../components/invoices/InvoicesOverview.vue";
import InvoiceEditor from "../components/invoices/InvoiceEditor.vue";
import Offers from "../views/OffersView.vue";
import OffersOverview from "../components/offers/OffersOverview.vue";
import OfferEditor from "../components/offers/OfferEditor.vue";
import Receipts from "../views/ReceiptsView.vue";
import ReceiptsOverview from "../components/receipts/ReceiptsOverview.vue";
import ReceiptEditor from "../components/receipts/ReceiptEditor.vue";
import Reporting from "../views/ReportingView.vue";

const router = createRouter({
  // We use hash mode. I haven't found the correct way to configure Spring boot otherwise yet.
  // Also it doesn't matter since hash mode is only bad for SEO and stuff which we won't need anyway.
  history: createWebHashHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: "/",
      name: "Dashboard",
      component: Home,
    },
    {
      path: "/contacts",
      name: "Contacts",
      component: Contacts,
    },
    {
      path: "/offers",
      children: [
        {
          path: ":id",
          name: "OfferEditor",
          component: OfferEditor,
        },
        {
          path: ":id/revisions/:revision",
          name: "OfferEditorWithRevision",
          component: OfferEditor,
        },
        {
          path: "",
          name: "Offers",
          component: OffersOverview,
        },
      ],
      component: Offers,
    },
    {
      path: "/invoices",
      children: [
        {
          path: ":id",
          name: "InvoiceEditor",
          component: InvoiceEditor,
        },
        {
          path: "",
          name: "Invoices",
          component: InvoicesOverview,
        },
      ],
      component: Invoices,
    },
    {
      path: "/receipts",
      children: [
        {
          path: ":id",
          name: "ReceiptEditor",
          component: ReceiptEditor,
        },
        {
          path: "",
          name: "Receipts",
          component: ReceiptsOverview,
        },
      ],
      component: Receipts,
    },
    {
      path: "/reporting",
      name: "Reporting",
      component: Reporting,
    },
  ]
})

export default router
