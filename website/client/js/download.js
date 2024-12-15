function toggleDropdown(id) {
    const dropdown = document.getElementById(id);
    const header = dropdown.previousElementSibling;
    dropdown.style.display = dropdown.style.display === 'block' ? 'none' : 'block';
    header.classList.toggle('active');
}

function copyCode(button) {
    const code = button.previousElementSibling.innerText;
    navigator.clipboard.writeText(code).catch(() => alert("Failed to copy to clipboard"));
}