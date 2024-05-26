const interval = setInterval(function() {
    let count = document.getElementById('count');
    let data = fetch("/count").then((data) => data.json()).then((data) => count.textContent = data.count).await;
}, 1000);