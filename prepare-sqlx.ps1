get-content prepare-sqlx.env | foreach {
    $name, $value = $_.split('=')
    $value = $value -replace 'export ', ''
    set-content env:\$name $value
}
