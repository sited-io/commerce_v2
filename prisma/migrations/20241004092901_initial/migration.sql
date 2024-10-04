-- CreateEnum
CREATE TYPE "OfferTypeKey" AS ENUM ('PHYSICAL', 'DIGITAL');

-- CreateEnum
CREATE TYPE "PriceTypeKey" AS ENUM ('ONE_TIME', 'RECURRING');

-- CreateEnum
CREATE TYPE "ContentTypeKey" AS ENUM ('UNSPECIFIED', 'IMAGE');

-- CreateTable
CREATE TABLE "offers" (
    "offer_id" UUID NOT NULL,
    "owner" TEXT NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL,

    CONSTRAINT "offers_pkey" PRIMARY KEY ("offer_id")
);

-- CreateTable
CREATE TABLE "offer_details" (
    "offer_id" UUID NOT NULL,
    "owner" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    "description" TEXT,

    CONSTRAINT "offer_details_pkey" PRIMARY KEY ("offer_id")
);

-- CreateTable
CREATE TABLE "offer_types" (
    "offer_id" UUID NOT NULL,
    "owner" TEXT NOT NULL,
    "offer_type_key" "OfferTypeKey" NOT NULL,

    CONSTRAINT "offer_types_pkey" PRIMARY KEY ("offer_id")
);

-- CreateTable
CREATE TABLE "offer_prices" (
    "offer_id" UUID NOT NULL,
    "owner" TEXT NOT NULL,
    "unit_amount" INTEGER NOT NULL,
    "currency" TEXT NOT NULL,

    CONSTRAINT "offer_prices_pkey" PRIMARY KEY ("offer_id")
);

-- CreateTable
CREATE TABLE "PriceType" (
    "offer_id" UUID NOT NULL,
    "owner" TEXT NOT NULL,
    "price_type_key" "PriceTypeKey" NOT NULL,
    "recurring_interval" TEXT,
    "recurring_interval_count" INTEGER,
    "recurring_trial_period_days" INTEGER,

    CONSTRAINT "PriceType_pkey" PRIMARY KEY ("offer_id")
);

-- CreateTable
CREATE TABLE "shipping_rates" (
    "shipping_rate_id" UUID NOT NULL,
    "offer_id" UUID NOT NULL,
    "owner" TEXT NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "unit_amount" INTEGER NOT NULL,
    "currency" TEXT NOT NULL,
    "all_countries" BOOLEAN NOT NULL,
    "specific_countries" TEXT[],

    CONSTRAINT "shipping_rates_pkey" PRIMARY KEY ("shipping_rate_id")
);

-- CreateTable
CREATE TABLE "offer_images" (
    "offer_image_id" UUID NOT NULL,
    "offer_id" UUID NOT NULL,
    "file_id" UUID NOT NULL,
    "owner" TEXT NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "ordering" INTEGER NOT NULL,

    CONSTRAINT "offer_images_pkey" PRIMARY KEY ("offer_image_id")
);

-- CreateTable
CREATE TABLE "shops" (
    "shop_id" UUID NOT NULL,
    "owner" TEXT NOT NULL,
    "website_id" TEXT NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL,

    CONSTRAINT "shops_pkey" PRIMARY KEY ("shop_id")
);

-- CreateTable
CREATE TABLE "sub_files" (
    "file_id" UUID NOT NULL,
    "owner" TEXT NOT NULL,
    "content_type" "ContentTypeKey" NOT NULL,
    "file_name" TEXT NOT NULL,
    "file_url" TEXT,

    CONSTRAINT "sub_files_pkey" PRIMARY KEY ("file_id")
);

-- CreateTable
CREATE TABLE "_OfferToShop" (
    "A" UUID NOT NULL,
    "B" UUID NOT NULL
);

-- CreateIndex
CREATE UNIQUE INDEX "_OfferToShop_AB_unique" ON "_OfferToShop"("A", "B");

-- CreateIndex
CREATE INDEX "_OfferToShop_B_index" ON "_OfferToShop"("B");

-- AddForeignKey
ALTER TABLE "offer_details" ADD CONSTRAINT "offer_details_offer_id_fkey" FOREIGN KEY ("offer_id") REFERENCES "offers"("offer_id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "offer_types" ADD CONSTRAINT "offer_types_offer_id_fkey" FOREIGN KEY ("offer_id") REFERENCES "offers"("offer_id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "offer_prices" ADD CONSTRAINT "offer_prices_offer_id_fkey" FOREIGN KEY ("offer_id") REFERENCES "offers"("offer_id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "PriceType" ADD CONSTRAINT "PriceType_offer_id_fkey" FOREIGN KEY ("offer_id") REFERENCES "offer_prices"("offer_id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "shipping_rates" ADD CONSTRAINT "shipping_rates_offer_id_fkey" FOREIGN KEY ("offer_id") REFERENCES "offers"("offer_id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "offer_images" ADD CONSTRAINT "offer_images_file_id_fkey" FOREIGN KEY ("file_id") REFERENCES "sub_files"("file_id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "offer_images" ADD CONSTRAINT "offer_images_offer_id_fkey" FOREIGN KEY ("offer_id") REFERENCES "offers"("offer_id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "_OfferToShop" ADD CONSTRAINT "_OfferToShop_A_fkey" FOREIGN KEY ("A") REFERENCES "offers"("offer_id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "_OfferToShop" ADD CONSTRAINT "_OfferToShop_B_fkey" FOREIGN KEY ("B") REFERENCES "shops"("shop_id") ON DELETE CASCADE ON UPDATE CASCADE;
