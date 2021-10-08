module.exports = {
  css: {
    loaderOptions: {
      scss: {
        prependData: "@import '@/assets/scss/_variables.scss';",
      },
    },
  },
  devServer: {
    port: 8081,
    historyApiFallback: true,
    proxy: {
      "/api/*": {
        target: "http://localhost:8080",
        changeOrigin: true,
        headers: true,
      },
    },
  },
};
