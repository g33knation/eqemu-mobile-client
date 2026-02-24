
(function () {
    console.log("Teleporting...");
    if (window.teleport) {
        window.teleport(0, -2012, 2150.5);
    } else {
        console.error("Teleport function not found!");
    }
})();
