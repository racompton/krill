let username = 'admin@krill';

describe('OpenID Connect provider with custom logout URL', () => {
  it('Logout when logged in behaves as expected', () => {
    cy.visit('/')
    cy.url().should('not.include', Cypress.config('baseUrl'))
    cy.contains('Mock OpenID Connect login form')
    cy.get('input[name="username"]').clear().type(username)
    cy.get('input[name="userattr1"]').clear().type('role')         // a role is required to be able to login
    cy.get('input[name="userattrval1"]').clear().type('admin')
    cy.contains('Sign In').click()

    // We should end up back in the Krill UI
    cy.url().should('include', Cypress.config('baseUrl'))
    cy.contains('Sign In').should('not.exist')
    cy.get('#userinfo').click()
    cy.get('#userinfo_table').contains(username)
    cy.get('#userinfo_table').contains("role")

    // verify that the mock provider thinks the user is logged in
    cy.request('https://127.0.0.1:1818/test/is_user_logged_in?username=' + username).its('status').should('eq', 200)

    // logout
    cy.intercept('https://example.net/', 'custom logout page requested').as('getCustomLogoutURL')
    cy.get('.logout').click()

    // verify that we are directed to the custom logout URL stub
    cy.wait('@getCustomLogoutURL').its('response.statusCode').should('eq', 200)
    cy.url().should('eq', 'https://example.net/')

    // verify that the mock provider thinks the user is STILL logged in because due to the use of a custom logout URL
    // we deliberately did NOT tell the OpenID Connect mock provider that the user should be logged out
    cy.request('https://127.0.0.1:1818/test/is_user_logged_in?username=' + username).its('status').should('eq', 200)
  })
})