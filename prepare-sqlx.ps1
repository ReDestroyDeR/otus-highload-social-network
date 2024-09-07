get-content prepare-sqlx.env | foreach {
    $name, $value = $_.split('=')
    set-content env:\$name $value
}
